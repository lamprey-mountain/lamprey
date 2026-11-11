use std::{num::NonZeroU32, time::Duration};

use governor::Quota;
use http::{Method, StatusCode};

use crate::util::fnv::{fnv1a_u32, fnv1a_u64};

/// the buckets we ratelimit against
///
/// these names are internal only; the public `X-RateLimit-Bucket` header uses an
/// opaque id so clients don't couple to our naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Bucket {
    DefaultRead,
    DefaultWrite,
    MediaUpload,
    RoomRead,
    RoomWrite,
    ChannelRead,
    ChannelWrite,
    InitDm,
    InviteUse,
    Email,
    UserCreate,
}

/// extra cost applied after the response status is known
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Penalty {
    None,

    /// a newly created dm counts as five requests
    InitDm,

    /// 404s on enumeration-prone lookups count as fifteen requests
    Enumeration,
}

/// the bucket assignment for one request
#[derive(Debug, Clone, Copy)]
pub struct Route {
    pub bucket: Bucket,

    /// room/channel scope. `None` for everything else.
    ///
    /// room/channel ids are uuids, but ratelimit keys only need a compact,
    /// deterministic fingerprint. this is a hash of the id, not the id itself.
    pub scope: Option<u64>,

    pub penalty: Penalty,
}

impl Bucket {
    /// opaque, stable id for the `X-RateLimit-Bucket` header
    ///
    /// room/channel buckets include their scope so each room/channel gets a
    /// distinct bucket id.
    pub fn id(self, scope: Option<u64>) -> String {
        let hash = fnv1a_u32(
            self.as_ref()
                .bytes()
                .chain(scope.into_iter().flat_map(|scope| scope.to_le_bytes())),
        );

        format!("{hash:08x}")
    }

    /// per-user quota. ip and session limits are derived from this.
    pub fn quota(self) -> Quota {
        match self {
            // 100 requests / 5 seconds
            Self::DefaultRead | Self::RoomRead | Self::ChannelRead => {
                make_quota(100, Duration::from_millis(50))
            }

            // 20 requests / 5 seconds
            Self::DefaultWrite | Self::RoomWrite | Self::ChannelWrite => {
                make_quota(20, Duration::from_millis(250))
            }

            // chunk uploads can be frequent; keep them out of the write bucket
            Self::MediaUpload => make_quota(100, Duration::from_millis(50)),

            // 15 requests / 15 seconds
            Self::InitDm => make_quota(15, Duration::from_secs(1)),

            // 2 requests / 30 seconds
            Self::InviteUse => make_quota(2, Duration::from_secs(15)),

            // 3 requests / 60 seconds
            Self::Email => make_quota(3, Duration::from_secs(20)),

            // 2 requests / 60 seconds
            Self::UserCreate => make_quota(2, Duration::from_secs(30)),
        }
    }
}

impl Penalty {
    /// extra requests to charge after the response is known
    ///
    /// the initial request has already been counted as one.
    pub fn extra_cost(self, status: StatusCode) -> u32 {
        match self {
            Self::InitDm if status == StatusCode::CREATED => 4,
            Self::Enumeration if status == StatusCode::NOT_FOUND => 14,
            _ => 0,
        }
    }
}

impl Route {
    /// pick a bucket from the matched route template and the actual path
    ///
    /// `matched` is axum's `MatchedPath` (`/api/v1/room/{room_id}/...`), while
    /// `actual` is the raw request path used to extract room/channel ids.
    pub fn for_request(method: &Method, matched: &str, actual: &str) -> Self {
        let path = normalize_path(matched);
        let actual = normalize_path(actual);
        let read = matches!(*method, Method::GET | Method::HEAD);

        // init dm: POST /user/@self/dm/{target_id}
        if method == Method::POST && path.starts_with("/user/@self/dm/") {
            return Self {
                bucket: Bucket::InitDm,
                scope: None,
                penalty: Penalty::InitDm,
            };
        }

        // create user: POST /user (or the legacy admin register route)
        // oauth dynamic client registration also creates a user
        if method == Method::POST
            && (path == "/user" || path.ends_with("/register-user") || path == "/oauth/register")
        {
            return Self {
                bucket: Bucket::UserCreate,
                scope: None,
                penalty: Penalty::None,
            };
        }

        // email sending / verification routes
        if path.contains("/email") && !read {
            return Self {
                bucket: Bucket::Email,
                scope: None,
                penalty: Penalty::None,
            };
        }

        // invite resolve: GET /invite/{code}
        if read && is_single_segment(path, "/invite/") {
            return Self {
                bucket: Bucket::DefaultRead,
                scope: None,
                penalty: Penalty::Enumeration,
            };
        }

        // invite use: POST /invite/{code}
        if method == Method::POST && is_single_segment(path, "/invite/") {
            return Self {
                bucket: Bucket::InviteUse,
                scope: None,
                penalty: Penalty::None,
            };
        }

        // user get: GET /user/{id}
        if read && is_user_get(path) {
            return Self {
                bucket: Bucket::DefaultRead,
                scope: None,
                penalty: Penalty::Enumeration,
            };
        }

        if !read && path.starts_with("/internal/media-upload/") {
            return Self {
                bucket: Bucket::MediaUpload,
                scope: None,
                penalty: Penalty::None,
            };
        }

        if let Some(scope) = scope_from(actual, "/room/") {
            return Self {
                bucket: if read {
                    Bucket::RoomRead
                } else {
                    Bucket::RoomWrite
                },
                scope: Some(scope),
                penalty: Penalty::None,
            };
        }

        if let Some(scope) = scope_from(actual, "/channel/") {
            return Self {
                bucket: if read {
                    Bucket::ChannelRead
                } else {
                    Bucket::ChannelWrite
                },
                scope: Some(scope),
                penalty: Penalty::None,
            };
        }

        Self {
            bucket: if read {
                Bucket::DefaultRead
            } else {
                Bucket::DefaultWrite
            },
            scope: None,
            penalty: Penalty::None,
        }
    }
}

fn normalize_path(path: &str) -> &str {
    for prefix in ["/api/v1", "/api/v2"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return rest;
        }
    }
    path
}

fn is_single_segment(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
}

fn is_user_get(path: &str) -> bool {
    let Some(id) = path.strip_prefix("/user/") else {
        return false;
    };

    !id.is_empty() && !id.contains('/') && id != "@self" && id != "search"
}

fn scope_from(path: &str, prefix: &str) -> Option<u64> {
    let id = path.strip_prefix(prefix)?.split('/').next()?;
    if id.is_empty() || id.starts_with('{') {
        return None;
    }
    Some(scope_id(id))
}

/// FNV-1a 64-bit hash of a room/channel id.
///
/// room/channel ids are uuids, but ratelimit keys only need a compact,
/// deterministic fingerprint. this is not a database id and is not used for
/// lookups; collisions are accepted as a 64-bit-key tradeoff.
fn scope_id(id: &str) -> u64 {
    fnv1a_u64(id.bytes())
}

/// `Quota::new` is deprecated and its argument order is easy to misread
/// (`replenish_all_per` vs `replenish_1_per`). `with_period` + `allow_burst`
/// is the constructor governor recommends, and expresses the same quota.
fn make_quota(burst: u32, replenish: Duration) -> Quota {
    Quota::with_period(replenish)
        .expect("nonzero replenish interval")
        .allow_burst(NonZeroU32::new(burst).expect("nonzero burst"))
}

/// multiply both the burst size and the replenish rate
pub fn scale(quota: Quota, factor: u32) -> Quota {
    let burst = quota.burst_size().get().saturating_mul(factor).max(1);
    let nanos = (quota.replenish_interval().as_nanos() / factor as u128).max(1) as u64;

    make_quota(burst, Duration::from_nanos(nanos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_upload_chunks_use_a_separate_bucket() {
        let route = Route::for_request(
            &Method::PATCH,
            "/api/v1/internal/media-upload/{media_id}",
            "/api/v1/internal/media-upload/x",
        );
        assert_eq!(route.bucket, Bucket::MediaUpload);
    }

    #[test]
    fn media_create_stays_on_the_default_bucket() {
        for (method, path) in [
            (Method::POST, "/api/v1/media"),
            (Method::PUT, "/api/v1/media/{media_id}/done"),
        ] {
            let route = Route::for_request(&method, path, path);
            assert_eq!(route.bucket, Bucket::DefaultWrite);
        }
    }

    #[test]
    fn media_reads_stay_on_the_default_bucket() {
        let route = Route::for_request(&Method::GET, "/api/v1/media/{media_id}", "/api/v1/media/x");
        assert_eq!(route.bucket, Bucket::DefaultRead);
    }
}
