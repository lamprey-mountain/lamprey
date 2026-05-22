use std::{net::IpAddr, sync::Arc};

use lamprey_backend_core::Result;

use crate::ServerStateInner;

/// ip address service
pub struct ServiceIps {
    state: Arc<ServerStateInner>,
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

        Self { state, reader }
    }

    pub fn lookup(&self, addr: IpAddr) -> Result<Option<IpInfo>> {
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

        Ok(Some(IpInfo {
            location,
            time_zone: info.location.time_zone.map(String::from),
            country_code: info.country.iso_code.map(String::from),
            country_name: info.country.names.english.map(String::from),
            city_name: info.city.names.english.map(String::from),
            is_in_european_union: info.country.is_in_european_union.unwrap_or_default(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct IpInfo {
    /// approximate location of this ip address
    pub location: Option<IpLocation>,

    /// iana time zone identifier, eg. "America/New_York"
    pub time_zone: Option<String>,

    /// two-character iso code, eg. "US", "DE"
    pub country_code: Option<String>,

    /// country name in english
    pub country_name: Option<String>,

    /// city name in english
    pub city_name: Option<String>,

    /// whether this ip is in the european union
    pub is_in_european_union: bool,
}

#[derive(Debug, Clone)]
pub struct IpLocation {
    /// approximate latitude of this ip address
    pub latitude: f64,

    /// approximate longitude of this ip address
    pub longitude: f64,

    /// estimate of location accuracy in kilometers
    pub accuracy_radius: Option<u16>,
}

impl IpLocation {
    /// calculate distance in kilometers using the haversine formula.
    // TODO: verify this is correct
    pub fn distance_to(&self, other: &Self) -> f64 {
        let earth_radius_km = 6371.0;

        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lon = (other.longitude - self.longitude).to_radians();

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();

        let a = (d_lat / 2.0).sin().powi(2) + (d_lon / 2.0).sin().powi(2) * lat1.cos() * lat2.cos();
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        earth_radius_km * c
    }
}
