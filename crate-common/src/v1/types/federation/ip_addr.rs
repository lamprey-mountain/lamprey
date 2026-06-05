#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// server information about an ip address
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IpInfo2 {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub metadata: IpMetadata,

    /// whether this ip address is banned
    pub banned: bool,
}

// /// a cidr range
// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct IpCidr(pub String);

/// metadata about an ip address
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IpMetadata {
    /// the ip address
    pub addr: String,

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

/// the approximate location of an ip address
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
