use std::{net::IpAddr, sync::Arc};

use common::v1::types::federation::ip_addr::{IpLocation, IpMetadata};
use lamprey_backend_core::Result;

use crate::ServerStateInner;

/// ip address service
pub struct ServiceIps {
    _state: Arc<ServerStateInner>,
    reader: Option<maxminddb::Reader<Vec<u8>>>,
}

impl ServiceIps {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let get_reader = || {
            let path = state.config.mmdb_path.as_deref()?;
            let reader = maxminddb::Reader::open_readfile(path).ok()?;
            Some(reader)
        };

        let reader = get_reader();

        Self {
            _state: state,
            reader,
        }
    }

    pub fn lookup(&self, addr: IpAddr) -> Result<Option<IpMetadata>> {
        let Some(reader) = &self.reader else {
            return Ok(None);
        };

        let info: Option<maxminddb::geoip2::City> = reader.lookup(addr)?.decode()?;
        let Some(info) = info else {
            return Ok(None);
        };

        let location = match (info.location.latitude, info.location.longitude) {
            (Some(lat), Some(lng)) => Some(IpLocation {
                latitude: lat,
                longitude: lng,
                accuracy_radius: info.location.accuracy_radius,
            }),
            _ => None,
        };

        Ok(Some(IpMetadata {
            addr: addr.to_string(),
            location,
            time_zone: info.location.time_zone.map(String::from),
            country_code: info.country.iso_code.map(String::from),
            country_name: info.country.names.english.map(String::from),
            city_name: info.city.names.english.map(String::from),
            is_in_european_union: info.country.is_in_european_union.unwrap_or_default(),
        }))
    }
}
