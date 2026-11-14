use std::{collections::HashSet, net::IpAddr};

use ipnet::IpNet;
use lamprey::v1::types::{ApplicationId, UserId};
use serde::{Deserialize, Serialize};

/// http ratelimit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRatelimit {
    /// disable ratelimiting entirely
    #[serde(default)]
    pub disabled: bool,

    /// multiplier applied to every bucket's burst and replenish rate
    ///
    /// for example, `4` allows four times as many requests in the same window.
    #[serde(default = "default_multiplier")]
    pub multiplier: u32,

    // TODO: store exempt users/apps/ips in the database and make them
    // configurable at runtime
    /// users exempt from all ratelimits, including ip-based ones
    ///
    /// exemption is only known once the request authenticates, so requests that
    /// never do (ie. routes without auth extractors) are still subject to the ip
    /// limit.
    #[serde(default)]
    pub exempt_users: HashSet<UserId>,

    /// applications exempt from all ratelimits, including ip-based ones
    ///
    /// every puppet a bridge app acts as (via `x-puppet-id`) is exempt too.
    #[serde(default)]
    pub exempt_apps: HashSet<ApplicationId>,

    /// ip addresses and cidr networks exempt from all ratelimits
    #[serde(default)]
    pub exempt_ips: Vec<IpNet>,
}

fn default_multiplier() -> u32 {
    1
}

impl Default for ConfigRatelimit {
    fn default() -> Self {
        Self {
            disabled: false,
            multiplier: default_multiplier(),
            exempt_users: HashSet::new(),
            exempt_apps: HashSet::new(),
            exempt_ips: Vec::new(),
        }
    }
}

impl ConfigRatelimit {
    /// the multiplier to apply to every quota; `0` is treated as `1`
    pub fn factor(&self) -> u32 {
        self.multiplier.max(1)
    }

    pub fn is_exempt_ip(&self, ip: IpAddr) -> bool {
        self.exempt_ips.iter().any(|net| net.contains(&ip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplier_zero_is_treated_as_one() {
        assert_eq!(ConfigRatelimit::default().factor(), 1);

        let config = ConfigRatelimit {
            multiplier: 0,
            ..Default::default()
        };
        assert_eq!(config.factor(), 1);

        let config = ConfigRatelimit {
            multiplier: 4,
            ..Default::default()
        };
        assert_eq!(config.factor(), 4);
    }

    #[test]
    fn exempt_ips_match_addresses_and_networks() {
        let config = ConfigRatelimit {
            exempt_ips: vec![
                "10.0.0.0/8".parse().expect("valid network"),
                "127.0.0.1/32".parse().expect("valid network"),
            ],
            ..Default::default()
        };

        assert!(config.is_exempt_ip("10.1.2.3".parse().expect("valid address")));
        assert!(config.is_exempt_ip("127.0.0.1".parse().expect("valid address")));
        assert!(!config.is_exempt_ip("192.168.0.1".parse().expect("valid address")));
    }
}
