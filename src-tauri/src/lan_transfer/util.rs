//! 局域网互传的通用小工具：文件名清洗、唯一路径、MIME 推断与目录校验。

use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use uuid::Uuid;

// 虚拟网卡名称前缀：容器、虚拟机、隧道与点对点接口的地址不适合作为扫码页地址。
const VIRTUAL_INTERFACE_PREFIXES: &[&str] = &[
    "docker",
    "veth",
    "br-",
    "virbr",
    "vmnet",
    "vboxnet",
    "tun",
    "tap",
    "wg",
    "utun",
    "awdl",
    "llw",
    "anpi",
    "zt",
    "tailscale",
    "ham",
    "pan",
    "vmnic",
];

// 判断网卡名是否属于虚拟/隧道接口。
pub(crate) fn is_virtual_interface(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    VIRTUAL_INTERFACE_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

// 给候选地址打分：数值越小越优先，地址类别优先于网卡类型。
fn candidate_rank(ip: IpAddr, interface_name: Option<&str>) -> (u8, u8) {
    let address_rank = match ip {
        IpAddr::V4(address) if address.is_private() => 0,
        IpAddr::V4(address) if address.is_link_local() => 3,
        IpAddr::V4(_) => 1,
        IpAddr::V6(address) if address.is_unicast_link_local() => 3,
        IpAddr::V6(_) => 2,
    };

    let interface_rank = match interface_name {
        Some(name) if is_virtual_interface(name) => 2,
        Some(_) => 0,
        None => 1,
    };

    (address_rank, interface_rank)
}

// 从候选地址里选出最适合放进二维码的地址；全部不理想时仍返回最优的一个。
fn preferred_lan_address_with<'a>(
    candidates: &[SocketAddr],
    interface_name_of: impl Fn(IpAddr) -> Option<&'a str>,
) -> Option<SocketAddr> {
    candidates
        .iter()
        .enumerate()
        .min_by_key(|(index, candidate)| {
            let (address_rank, interface_rank) =
                candidate_rank(candidate.ip(), interface_name_of(candidate.ip()));
            (address_rank, interface_rank, *index)
        })
        .map(|(_, candidate)| *candidate)
}

// 扫描页地址：优先真实网卡上的私有 IPv4，避免二维码指向 docker0 / VPN 等虚拟网卡。
pub(crate) fn preferred_lan_address(candidates: &[SocketAddr]) -> Option<SocketAddr> {
    let interfaces = if_addrs::get_if_addrs()
        .map(|interfaces| {
            interfaces
                .into_iter()
                .map(|interface| (interface.ip(), interface.name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    preferred_lan_address_with(candidates, |ip| {
        interfaces
            .iter()
            .find(|(address, _)| *address == ip)
            .map(|(_, name)| name.as_str())
    })
}

// 返回当前时间的毫秒时间戳，用于事件与文件名。
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default()
}

// 清洗对端提供的文件名：去掉路径分隔符与非法字符，避免越权写盘。
pub(crate) fn sanitize_file_name(value: &str) -> String {
    let name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("transfer-file")
        .trim();
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .trim()
        .to_string();

    if sanitized.is_empty() {
        "transfer-file".into()
    } else {
        sanitized.chars().take(180).collect()
    }
}

// 在目录内生成不冲突的文件路径，重名时追加序号。
pub(crate) fn unique_file_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("transfer-file");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let next_name = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem} ({index}).{extension}"),
            _ => format!("{stem} ({index})"),
        };
        let next = dir.join(next_name);
        if !next.exists() {
            return next;
        }
    }

    dir.join(format!("{stem}-{}", Uuid::new_v4()))
}

// 根据扩展名推断 MIME，未知类型回退为二进制流。
pub(crate) fn infer_mime_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "txt" | "log" | "md" => "text/plain",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .into()
}

// 校验接收目录可用：必须存在、是目录且可写。
pub(crate) fn validate_download_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("lan_transfer_download_dir_missing");
    }
    if !path.is_dir() {
        anyhow::bail!("lan_transfer_download_dir_not_directory");
    }
    let probe = path.join(format!(".power-paste-write-test-{}", Uuid::new_v4()));
    fs::write(&probe, b"ok").context("lan_transfer_download_dir_not_writable")?;
    fs::remove_file(&probe).context("lan_transfer_download_dir_cleanup_failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, SocketAddr},
        path::Path,
    };

    use super::{
        infer_mime_type, is_virtual_interface, preferred_lan_address_with, sanitize_file_name,
        unique_file_path,
    };

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("../a:b?.txt"), "a_b_.txt");
        assert_eq!(sanitize_file_name("..."), "transfer-file");
        assert_eq!(sanitize_file_name("dir/photo.png"), "photo.png");
    }

    #[test]
    fn keeps_unique_file_name_when_available() {
        let dir = std::env::temp_dir();
        let path = unique_file_path(&dir, "power-paste-unique-name-test.txt");
        assert!(path.ends_with("power-paste-unique-name-test.txt"));
    }

    #[test]
    fn infers_common_mime_types() {
        assert_eq!(infer_mime_type(Path::new("a/b/c.png")), "image/png");
        assert_eq!(infer_mime_type(Path::new("notes.txt")), "text/plain");
        assert_eq!(
            infer_mime_type(Path::new("data.bin")),
            "application/octet-stream"
        );
    }

    #[test]
    fn detects_virtual_interface_names() {
        assert!(is_virtual_interface("docker0"));
        assert!(is_virtual_interface("vEthernet (WSL)"));
        assert!(is_virtual_interface("utun3"));
        assert!(is_virtual_interface("tailscale0"));
        assert!(!is_virtual_interface("eth0"));
        assert!(!is_virtual_interface("en0"));
        assert!(!is_virtual_interface("WLAN"));
    }

    #[test]
    fn prefers_a_physical_interface_over_a_virtual_one() {
        // 虚拟网卡排在前面时仍然要选中真实网卡，否则二维码会指向容器/虚拟机地址。
        let docker = SocketAddr::from(([172, 17, 0, 1], 53317));
        let wifi = SocketAddr::from(([192, 168, 1, 20], 53317));

        let picked = preferred_lan_address_with(&[docker, wifi], |ip| match ip {
            IpAddr::V4(address) if address.octets()[1] == 17 => Some("docker0"),
            _ => Some("en0"),
        });

        assert_eq!(picked, Some(wifi));
    }

    #[test]
    fn prefers_private_ipv4_over_other_addresses() {
        let carrier_nat = SocketAddr::from(([100, 64, 0, 7], 53317));
        let lan = SocketAddr::from(([10, 0, 0, 5], 53317));

        let picked = preferred_lan_address_with(&[carrier_nat, lan], |_| Some("eth0"));

        assert_eq!(picked, Some(lan));
    }

    #[test]
    fn keeps_the_first_candidate_when_every_address_is_unideal() {
        // 只有链路本地地址时仍然返回结果，保证扫码页不会因为挑不出地址而整体失败。
        let link_local = SocketAddr::from(([169, 254, 12, 34], 53317));

        assert_eq!(
            preferred_lan_address_with(&[link_local], |_| Some("eth0")),
            Some(link_local)
        );
    }

    #[test]
    fn returns_none_without_candidates() {
        let empty: Vec<SocketAddr> = Vec::new();
        assert_eq!(preferred_lan_address_with(&empty, |_| Some("eth0")), None);
    }
}
