//! random utilities

use std::net::IpAddr;

use anyhow::{anyhow, Result};
use systemstat::{Platform, System};
use tracing::debug;

pub fn select_host_address_ipv4(host_ip: Option<&str>) -> Result<IpAddr> {
    if let Some(ip) = host_ip {
        if let Ok(addr) = ip.parse() {
            debug!("using configured ipv4 addr {addr}");
            return Ok(addr);
        }
    }

    let system = System::new();
    let networks = system
        .networks()
        .map_err(|e| anyhow!("failed to list network interfaces: {}", e))?;

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() && !v.is_private() {
                    debug!("selected ipv4 addr {v}");
                    return Ok(IpAddr::V4(v));
                }
            }
        }
    }

    Err(anyhow!("Found no usable ipv4 network interface"))
}

pub fn select_host_address_ipv6(host_ip: Option<&str>) -> Result<IpAddr> {
    if let Some(ip) = host_ip {
        if let Ok(addr) = ip.parse() {
            debug!("using configured ipv6 addr {addr}");
            return Ok(addr);
        }
    }
    let system = System::new();
    let networks = system
        .networks()
        .map_err(|e| anyhow!("failed to list network interfaces: {}", e))?;

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V6(v) = n.addr {
                if !v.is_loopback()
                    && !v.is_unicast_link_local()
                    && !v.is_multicast()
                    && !v.is_unique_local()
                {
                    debug!("selected ipv6 addr {v}");
                    return Ok(IpAddr::V6(v));
                }
            }
        }
    }

    Err(anyhow!("Found no usable ipv6 network interface"))
}

pub fn extract_stun_ufrag(data: &[u8]) -> Option<String> {
    if data.len() < 24 {
        return None;
    }

    // Skip header
    let mut pos = 20;

    while pos + 4 <= data.len() {
        let attr_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let attr_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if attr_type == 0x0006 {
            // USERNAME attribute
            if pos + attr_len <= data.len() {
                let username = String::from_utf8_lossy(&data[pos..pos + attr_len]);
                // ICE username is usually "local_ufrag:remote_ufrag" or just "local_ufrag"
                return Some(username.split(':').next().unwrap_or(&username).to_string());
            }
        }

        // Attributes are padded to 4 bytes
        pos += (attr_len + 3) & !3;
    }
    None
}
