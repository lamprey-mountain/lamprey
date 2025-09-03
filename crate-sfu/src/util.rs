use std::net::IpAddr;

use systemstat::{Platform, System};
use tracing::debug;

pub fn select_host_address_ipv4(host_ip: Option<&str>) -> IpAddr {
    if let Some(ip) = host_ip {
        if let Ok(addr) = ip.parse() {
            debug!("using configured ipv4 addr {addr}");
            return addr;
        }
    }

    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() && !v.is_private() {
                    debug!("selected ipv4 addr {v}");
                    return IpAddr::V4(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}

pub fn select_host_address_ipv6(host_ip: Option<&str>) -> IpAddr {
    if let Some(ip) = host_ip {
        if let Ok(addr) = ip.parse() {
            debug!("using configured ipv6 addr {addr}");
            return addr;
        }
    }
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V6(v) = n.addr {
                if !v.is_loopback()
                    && !v.is_unicast_link_local()
                    && !v.is_multicast()
                    && !v.is_unique_local()
                {
                    debug!("selected ipv6 addr {v}");
                    return IpAddr::V6(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}
