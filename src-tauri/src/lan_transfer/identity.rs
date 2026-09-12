//! 本机在 LocalSend 网络中的身份：自签证书、指纹与设备信息。
//!
//! 证书与私钥保存在应用数据目录，指纹持久化后对端才会持续把本机当作同一台设备。

use std::{fs, path::Path};

use anyhow::{Context, Result};
use localsend::{
    crypto::cert::fingerprint_from_cert_der,
    http::{dto_v2::RegisterDtoV2, server::TlsConfig, state::ClientInfo},
    model::discovery::{DeviceType, ProtocolType, PROTOCOL_VERSION_V2},
    multicast::MulticastDevice,
};

// 身份文件名：证书与私钥按 PEM 顺序存放在同一个文件里。
pub(crate) const IDENTITY_FILE: &str = "localsend-identity.pem";

// 对端展示的设备型号，用于区分 power-paste 与官方客户端。
pub(crate) const DEVICE_MODEL: &str = "Power Paste";

pub(crate) struct LanIdentity {
    pub(crate) alias: String,
    pub(crate) port: u16,
    pub(crate) cert_pem: String,
    pub(crate) key_pem: String,
    pub(crate) fingerprint: String,
}

impl LanIdentity {
    // 从目录读取身份文件；不存在时生成并落盘新身份。
    pub(crate) fn load_or_generate(dir: &Path, alias: String, port: u16) -> Result<Self> {
        let path = dir.join(IDENTITY_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => Self::from_pem(&text, alias, port).with_context(|| {
                format!(
                    "invalid identity file: {} (delete it to generate a new identity; \
                     other devices will then see this device as unpaired)",
                    path.display()
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let identity = Self::generate(alias, port)?;
                identity
                    .save(&path)
                    .with_context(|| format!("could not save {}", path.display()))?;
                Ok(identity)
            }
            Err(error) => Err(error).context(format!("could not read {}", path.display())),
        }
    }

    fn from_pem(text: &str, alias: String, port: u16) -> Result<Self> {
        let blocks = pem::parse_many(text)?;
        let cert = blocks
            .iter()
            .find(|block| block.tag() == "CERTIFICATE")
            .context("missing CERTIFICATE block")?;
        let key = blocks
            .iter()
            .find(|block| block.tag().ends_with("PRIVATE KEY"))
            .context("missing PRIVATE KEY block")?;

        Ok(Self {
            alias,
            port,
            fingerprint: fingerprint_from_cert_der(cert.contents()),
            cert_pem: pem::encode(cert),
            key_pem: pem::encode(key),
        })
    }

    fn generate(alias: String, port: u16) -> Result<Self> {
        let cert = localsend::crypto::cert::generate_self_signed()?;
        Ok(Self {
            alias,
            port,
            fingerprint: cert.fingerprint,
            cert_pem: cert.certificate_pem,
            key_pem: cert.private_key_pem,
        })
    }

    fn save(&self, path: &Path) -> Result<()> {
        let contents = format!("{}{}", self.cert_pem, self.key_pem);

        #[cfg(unix)]
        {
            // 文件内含私钥，仅允许属主读写。
            use std::{io::Write, os::unix::fs::OpenOptionsExt};
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?;
            file.write_all(contents.as_bytes())?;
        }
        #[cfg(not(unix))]
        fs::write(path, contents)?;
        Ok(())
    }

    pub(crate) fn tls_config(&self) -> TlsConfig {
        TlsConfig {
            cert: self.cert_pem.clone(),
            private_key: self.key_pem.clone(),
        }
    }

    pub(crate) fn client_info(&self) -> ClientInfo {
        ClientInfo {
            alias: self.alias.clone(),
            version: PROTOCOL_VERSION_V2.to_string(),
            device_model: Some(DEVICE_MODEL.to_string()),
            device_type: Some(DeviceType::Desktop),
            token: self.fingerprint.clone(),
        }
    }

    pub(crate) fn register_dto(&self, protocol: ProtocolType) -> RegisterDtoV2 {
        RegisterDtoV2 {
            alias: self.alias.clone(),
            version: PROTOCOL_VERSION_V2.to_string(),
            device_model: Some(DEVICE_MODEL.to_string()),
            device_type: Some(DeviceType::Desktop),
            fingerprint: self.fingerprint.clone(),
            port: self.port,
            protocol,
            download: false,
        }
    }

    // 组装对外广播的设备信息；协议与下载能力随扫码页模式变化。
    pub(crate) fn multicast_device(
        &self,
        protocol: ProtocolType,
        download: bool,
    ) -> MulticastDevice {
        MulticastDevice {
            alias: self.alias.clone(),
            version: PROTOCOL_VERSION_V2.to_string(),
            device_model: Some(DEVICE_MODEL.to_string()),
            device_type: Some(DeviceType::Desktop),
            fingerprint: self.fingerprint.clone(),
            port: self.port,
            protocol,
            download,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LanIdentity;

    #[test]
    fn regenerates_the_same_fingerprint_from_the_saved_file() {
        let dir = std::env::temp_dir().join(format!("power-paste-identity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let _ = std::fs::remove_file(dir.join(super::IDENTITY_FILE));

        let first = LanIdentity::load_or_generate(&dir, "Unit Test".into(), 53317)
            .expect("generate identity");
        let second = LanIdentity::load_or_generate(&dir, "Unit Test".into(), 53317)
            .expect("reload identity");

        assert_eq!(first.fingerprint, second.fingerprint);
        assert!(!first.cert_pem.is_empty());
        assert!(!first.key_pem.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
