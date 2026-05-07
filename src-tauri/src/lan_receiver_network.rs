use std::{
    net::{Ipv4Addr, UdpSocket},
    process::Command,
};

use anyhow::Result;
use qrcode::{render::svg, QrCode};

pub(crate) fn local_lan_ip() -> Result<String> {
    let mut candidates = local_ipv4_candidates();

    if let Ok(ip) = default_route_ipv4() {
        candidates.push(ip);
    }

    candidates
        .into_iter()
        .filter(|ip| usable_lan_ipv4(*ip))
        .max_by_key(|ip| lan_ipv4_score(*ip))
        .map(|ip| ip.to_string())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve local lan ip"))
}

fn default_route_ipv4() -> Result<Ipv4Addr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0))?;
    socket.connect(("8.8.8.8", 80))?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(ip) => Ok(ip),
        std::net::IpAddr::V6(_) => anyhow::bail!("default route resolved to ipv6"),
    }
}

fn local_ipv4_candidates() -> Vec<Ipv4Addr> {
    platform_ipv4_candidates()
        .into_iter()
        .fold(Vec::new(), |mut candidates, ip| {
            if !candidates.contains(&ip) {
                candidates.push(ip);
            }
            candidates
        })
}

#[cfg(target_os = "windows")]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    let output = match command_output_text("ipconfig", &["/all"]) {
        Some(output) => output,
        None => return Vec::new(),
    };
    extract_windows_ipv4_candidates(&output)
}

#[cfg(target_os = "macos")]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    command_ipv4_candidates("ifconfig", &[])
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    let candidates = command_ipv4_candidates("ip", &["-4", "addr"]);
    if candidates.is_empty() {
        command_ipv4_candidates("ifconfig", &[])
    } else {
        candidates
    }
}

#[cfg(not(any(windows, target_os = "macos", unix)))]
fn platform_ipv4_candidates() -> Vec<Ipv4Addr> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn command_ipv4_candidates(program: &str, args: &[&str]) -> Vec<Ipv4Addr> {
    command_output_text(program, args)
        .map(|text| extract_ipv4_candidates(&text))
        .unwrap_or_default()
}

fn command_output_text(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) fn extract_ipv4_candidates(text: &str) -> Vec<Ipv4Addr> {
    let mut candidates = Vec::new();
    for token in text.split(|value: char| !(value.is_ascii_digit() || value == '.')) {
        let Ok(ip) = token.parse::<Ipv4Addr>() else {
            continue;
        };
        if !candidates.contains(&ip) {
            candidates.push(ip);
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
pub(crate) fn extract_windows_ipv4_candidates(text: &str) -> Vec<Ipv4Addr> {
    let mut candidates = Vec::new();
    for line in text.lines().filter(|line| line.contains("IPv4")) {
        for ip in extract_ipv4_candidates(line) {
            if !candidates.contains(&ip) {
                candidates.push(ip);
            }
        }
    }
    candidates
}

pub(crate) fn usable_lan_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !ip.is_unspecified()
        && !(octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        && octets[3] != 0
        && octets[3] != 255
}

pub(crate) fn lan_ipv4_score(ip: Ipv4Addr) -> i32 {
    let octets = ip.octets();
    let host_score = match octets[3] {
        1 => -50,
        2..=9 => -10,
        _ => i32::from(octets[3]).min(40),
    };
    if octets[0] == 192 && octets[1] == 168 {
        return 400 + host_score;
    }
    if octets[0] == 10 {
        return 390 + host_score;
    }
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return 380 + host_score;
    }
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return 120 + host_score;
    }
    80 + host_score
}

pub(crate) fn build_qr_svg(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#1d232d"))
        .light_color(svg::Color("#ffffff"))
        .build())
}
