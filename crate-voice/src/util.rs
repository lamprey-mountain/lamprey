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
