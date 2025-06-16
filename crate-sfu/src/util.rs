use std::net::IpAddr;

use systemstat::{Platform, System};

pub fn select_host_address_ipv4() -> IpAddr {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() && !v.is_private() {
                    return IpAddr::V4(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}

// TODO: ipv6 support
// pub fn select_host_address_ipv6() -> IpAddr {
//     let system = System::new();
//     let networks = system.networks().unwrap();

//     for net in networks.values() {
//         for n in &net.addrs {
//             if let systemstat::IpAddr::V6(v) = n.addr {
//                 if !v.is_loopback() && !v.is_unicast_link_local() && !v.is_multicast() {
//                     return IpAddr::V6(v);
//                 }
//             }
//         }
//     }

//     panic!("Found no usable network interface");
// }
