mod buckets;

use std::{
    collections::HashMap,
    hash::Hash,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use crate::prelude::*;
use axum::{
    body::Body,
    extract::{ConnectInfo, MatchedPath},
    http::{HeaderMap, HeaderValue, Request, Response, header},
    middleware::Next,
    response::IntoResponse,
};
use buckets::{Bucket, Penalty, Route, scale};
use common::v1::types::{
    ApplicationId, SessionId, UserId,
    error::{ApiError, ErrorCode, Ratelimit, RatelimitScope},
};
use governor::{
    DefaultKeyedRateLimiter, Quota, RateLimiter as GovernorRateLimiter,
    clock::{Clock, DefaultClock},
    middleware::StateInformationMiddleware,
};
use kerosene_core::{config::ConfigRatelimit, types::auth::Identity};
use strum::IntoEnumIterator;

type KeyedLimiter<K> = DefaultKeyedRateLimiter<K, StateInformationMiddleware>;

/// a key in a keyed limiter.
///
/// the scope is part of the key so each room/channel gets its own bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    V4(Option<u64>, u32),
    V6_64(Option<u64>, u64),
    V6_48(Option<u64>, u64),
    User(Option<u64>, UserId),
    Session(Option<u64>, SessionId),
}

/// the result of a ratelimit check
#[derive(Debug, Clone, Copy)]
struct Decision {
    bucket: Bucket,
    scope: Option<u64>,
    limit: u32,
    remaining: u32,
    retry_after: Option<Duration>,
    ratelimit_scope: RatelimitScope,
}

/// the state shared between the middleware and auth extractors for one request
#[derive(Debug, Clone)]
pub struct RateLimitState {
    keys: Vec<Key>,
    decision: Decision,
    identity_checked: bool,
    exempt: bool,
    penalty: Penalty,
}

/// a slot the middleware injects into the request
pub type RateLimitSlot = Arc<Mutex<Option<RateLimitState>>>;

/// the identity details that decide whether a request is exempt and which
/// identity limits apply
#[derive(Debug, Clone, Copy, Default)]
pub struct RateLimitIdentity {
    /// the user making the request
    pub user_id: Option<UserId>,

    /// the session making the request
    pub session_id: Option<SessionId>,

    /// the oauth application the session belongs to
    pub app_id: Option<ApplicationId>,

    /// the user this request acts on behalf of, if it is a puppet request
    pub puppeteer_id: Option<UserId>,
}

impl RateLimitIdentity {
    /// build the ratelimit identity from an authenticated identity
    pub fn new(identity: &Identity) -> Self {
        let puppeteer_id = match identity {
            Identity::Puppet { puppeteer, .. } => Some(puppeteer.id),
            _ => None,
        };

        Self {
            user_id: identity.user_id(),
            session_id: identity.session().map(|session| session.id),
            app_id: identity.session().and_then(|session| session.app_id),
            puppeteer_id,
        }
    }
}

/// keyed limiters and quotas for one bucket kind.
///
/// ip addresses are grouped into v4, /64, and /48 keys, while users and sessions
/// are limited by their ids.
struct BucketLimiters {
    v4: KeyedLimiter<Key>,
    v6_64: KeyedLimiter<Key>,
    v6_48: KeyedLimiter<Key>,
    user: KeyedLimiter<Key>,
    session: KeyedLimiter<Key>,
    quota_user: Quota,
    quota_v4: Quota,
    quota_v6_64: Quota,
    quota_v6_48: Quota,
}

/// ratelimits for every bucket
struct RateLimiter {
    buckets: HashMap<Bucket, BucketLimiters>,
}

static GLOBAL: OnceLock<RateLimiter> = OnceLock::new();
static SETTINGS: OnceLock<ConfigRatelimit> = OnceLock::new();
static GC_STARTED: OnceLock<()> = OnceLock::new();

impl Decision {
    fn permit(bucket: Bucket, scope: Option<u64>, ratelimit_scope: RatelimitScope) -> Self {
        let limit = bucket.quota().burst_size().get();
        Self {
            bucket,
            scope,
            limit,
            remaining: limit,
            retry_after: None,
            ratelimit_scope,
        }
    }

    fn is_allowed(&self) -> bool {
        self.retry_after.is_none()
    }

    fn merge(self, other: Decision) -> Decision {
        if !self.is_allowed() {
            return self;
        }
        if !other.is_allowed() {
            return other;
        }

        let self_ratio = self.remaining as u64 * other.limit as u64;
        let other_ratio = other.remaining as u64 * self.limit as u64;
        if other_ratio < self_ratio {
            other
        } else {
            self
        }
    }

    /// add the ratelimit headers to a response
    fn write_headers(&self, headers: &mut HeaderMap) {
        let set = |headers: &mut HeaderMap, name: &'static str, value: &str| {
            if let Ok(value) = HeaderValue::from_str(value) {
                headers.insert(name, value);
            }
        };

        set(headers, "x-ratelimit-limit", &self.limit.to_string());
        set(
            headers,
            "x-ratelimit-remaining",
            &self.remaining.to_string(),
        );
        set(headers, "x-ratelimit-bucket", &self.bucket.id(self.scope));
        set(headers, "x-ratelimit-scope", self.ratelimit_scope.as_ref());

        if let Some(retry_after) = self.retry_after {
            set(
                headers,
                header::RETRY_AFTER.as_str(),
                &retry_after.as_secs().max(1).to_string(),
            );
        }
    }

    /// convert this into an api error
    fn into_error(self) -> ApiError {
        ApiError {
            ratelimit: Some(Ratelimit {
                retry_after: self.retry_after.unwrap_or_default().as_secs_f64(),
                bucket: self.bucket.id(self.scope),
                limit: self.limit,
                remaining: self.remaining,
                scope: self.ratelimit_scope,
            }),
            ..ApiError::from_code(ErrorCode::Ratelimit)
        }
    }
}

impl RateLimiter {
    fn new(config: &ConfigRatelimit) -> Self {
        let factor = config.factor();
        let buckets = Bucket::iter()
            .map(|bucket| (bucket, BucketLimiters::new(bucket, factor)))
            .collect();

        Self { buckets }
    }

    fn check_ip(
        &self,
        bucket: Bucket,
        scope: Option<u64>,
        ip: Option<IpAddr>,
    ) -> (Decision, Vec<Key>) {
        let limiters = &self.buckets[&bucket];

        let Some(ip) = ip else {
            return (
                Decision::permit(bucket, scope, RatelimitScope::Shared),
                Vec::new(),
            );
        };

        match ip.to_canonical() {
            IpAddr::V4(v4) => {
                let key = Key::V4(scope, u32::from(v4));
                let decision = check_keyed(
                    &limiters.v4,
                    &key,
                    bucket,
                    scope,
                    limiters.quota_v4,
                    RatelimitScope::Shared,
                );
                (decision, vec![key])
            }
            IpAddr::V6(v6) => {
                let bits = u128::from(v6);
                let key64 = Key::V6_64(scope, (bits >> 64) as u64);
                let key48 = Key::V6_48(scope, (bits >> 80) as u64);

                let d64 = check_keyed(
                    &limiters.v6_64,
                    &key64,
                    bucket,
                    scope,
                    limiters.quota_v6_64,
                    RatelimitScope::Shared,
                );
                let d48 = check_keyed(
                    &limiters.v6_48,
                    &key48,
                    bucket,
                    scope,
                    limiters.quota_v6_48,
                    RatelimitScope::Shared,
                );

                (d64.merge(d48), vec![key64, key48])
            }
        }
    }

    fn check_identity(
        &self,
        bucket: Bucket,
        scope: Option<u64>,
        user_id: Option<UserId>,
        session_id: Option<SessionId>,
    ) -> (Decision, Vec<Key>) {
        let limiters = &self.buckets[&bucket];

        let initial_scope = match (user_id.is_some(), session_id.is_some()) {
            (true, _) => RatelimitScope::User,
            (false, true) => RatelimitScope::Session,
            (false, false) => RatelimitScope::Global,
        };
        let mut decision = Decision::permit(bucket, scope, initial_scope);
        let mut keys = Vec::new();

        if let Some(user_id) = user_id {
            let key = Key::User(scope, user_id);
            decision = decision.merge(check_keyed(
                &limiters.user,
                &key,
                bucket,
                scope,
                limiters.quota_user,
                RatelimitScope::User,
            ));
            keys.push(key);
        }

        if let Some(session_id) = session_id {
            let key = Key::Session(scope, session_id);
            decision = decision.merge(check_keyed(
                &limiters.session,
                &key,
                bucket,
                scope,
                limiters.quota_user,
                RatelimitScope::Session,
            ));
            keys.push(key);
        }

        (decision, keys)
    }

    fn charge(&self, bucket: Bucket, key: Key, extra: u32) {
        let limiters = &self.buckets[&bucket];
        let limiter = match key {
            Key::V4(..) => &limiters.v4,
            Key::V6_64(..) => &limiters.v6_64,
            Key::V6_48(..) => &limiters.v6_48,
            Key::User(..) => &limiters.user,
            Key::Session(..) => &limiters.session,
        };
        charge_keyed(limiter, &key, extra);
    }

    /// evict keys that are indistinguishable from fresh state
    fn gc(&self) {
        for limiters in self.buckets.values() {
            limiters.v4.retain_recent();
            limiters.v6_64.retain_recent();
            limiters.v6_48.retain_recent();
            limiters.user.retain_recent();
            limiters.session.retain_recent();
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(settings())
    }
}

impl BucketLimiters {
    fn new(bucket: Bucket, factor: u32) -> Self {
        let quota_user = scale(bucket.quota(), factor);
        let quota_v4 = scale(quota_user, 4);
        let quota_v6_64 = scale(quota_user, 4);
        let quota_v6_48 = scale(quota_user, 64);

        Self {
            v4: keyed(quota_v4),
            v6_64: keyed(quota_v6_64),
            v6_48: keyed(quota_v6_48),
            user: keyed(quota_user),
            session: keyed(quota_user),
            quota_user,
            quota_v4,
            quota_v6_64,
            quota_v6_48,
        }
    }
}

/// middleware that applies ip limits and injects ratelimit headers
pub async fn middleware(mut req: Request<Body>, next: Next) -> Response<Body> {
    let config = settings();
    let ip = client_ip(&req);
    if config.disabled || ip.is_some_and(|ip| config.is_exempt_ip(ip)) {
        return next.run(req).await;
    }

    // prefer the route template (`/api/v1/user/{user_id}`) over the raw path so
    // bucket selection can't be confused by user-controlled path segments
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str())
        .unwrap_or_else(|| req.uri().path());
    let route = Route::for_request(req.method(), path, req.uri().path());
    let (decision, keys) = global().check_ip(route.bucket, route.scope, ip);

    let slot: RateLimitSlot = Arc::new(Mutex::new(Some(RateLimitState {
        keys,
        decision,
        identity_checked: false,
        exempt: false,
        penalty: route.penalty,
    })));
    req.extensions_mut().insert(slot.clone());

    // an authenticated request may belong to an exempt user, so the ip limit is
    // only enforced once the identity is known. requests without credentials
    // can never be exempt and are rejected before doing any work.
    if !decision.is_allowed() && !has_credentials(&req) {
        return reject(decision);
    }

    let res = next.run(req).await;

    let state = slot.lock().expect("ratelimit slot poisoned").clone();
    let Some(state) = state else {
        return res;
    };

    if state.exempt {
        return res;
    }

    // every api handler authenticates while extracting its request, which
    // enforces the ip decision before the handler body runs (see
    // `enforce_identity`). this only rejects routes that don't authenticate at
    // all (ie. webhooks), or requests whose authentication failed.
    if !state.decision.is_allowed() {
        return reject(state.decision);
    }

    let extra = state.penalty.extra_cost(res.status());
    if extra > 0 {
        for key in &state.keys {
            global().charge(state.decision.bucket, *key, extra);
        }
    }

    let mut res = res;
    state.decision.write_headers(res.headers_mut());
    res
}

fn reject(decision: Decision) -> Response<Body> {
    let mut res = Error::Api(Box::new(decision.into_error())).into_response();
    decision.write_headers(res.headers_mut());
    res
}

fn has_credentials(req: &Request<Body>) -> bool {
    req.headers().contains_key(header::AUTHORIZATION)
}

/// spawn the background task that evicts idle keys
pub fn spawn_gc() {
    if GC_STARTED.set(()).is_err() {
        return;
    }

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            global().gc();
        }
    });
}

/// get the ratelimit slot for a request, if the middleware installed one
pub fn slot(parts: &http::request::Parts) -> RateLimitSlot {
    parts
        .extensions
        .get::<RateLimitSlot>()
        .cloned()
        .unwrap_or_default()
}

/// apply the user/session part of a bucket after authentication
///
/// the middleware has already applied the ip limits for this request.
pub fn enforce_identity(
    slot: &RateLimitSlot,
    identity: &RateLimitIdentity,
) -> std::result::Result<(), ApiError> {
    let mut guard = slot.lock().expect("ratelimit slot poisoned");
    let Some(state) = guard.as_mut() else {
        return Ok(());
    };
    if state.identity_checked {
        return Ok(());
    }

    // exempt identities bypass every limit, including the ip limit that was
    // applied before the identity was known
    if is_exempt(identity) {
        state.identity_checked = true;
        state.exempt = true;
        state.keys.clear();
        return Ok(());
    }

    let user_id = identity.user_id;
    let session_id = identity.session_id;
    if user_id.is_none() && session_id.is_none() {
        // without a user or session there is nothing that could make this request
        // exempt, so the ip decision applies as-is. rejecting here instead of in
        // the middleware guarantees it happens before the handler does any work
        state.identity_checked = true;
        return if state.decision.is_allowed() {
            Ok(())
        } else {
            Err(state.decision.into_error())
        };
    }

    let (bucket, scope) = (state.decision.bucket, state.decision.scope);
    let (decision, keys) = global().check_identity(bucket, scope, user_id, session_id);
    state.keys.extend(keys);
    state.decision = state.decision.merge(decision);
    state.identity_checked = true;

    if state.decision.is_allowed() {
        Ok(())
    } else {
        Err(state.decision.into_error())
    }
}

/// apply the ratelimit settings from the server config
///
/// this should be called once at startup, before serving any requests. later
/// calls are ignored.
pub fn init(config: &ConfigRatelimit) {
    let _ = SETTINGS.set(config.clone());
}

/// whether every request made with this identity bypasses ratelimits
fn is_exempt(identity: &RateLimitIdentity) -> bool {
    let settings = settings();

    if identity
        .user_id
        .is_some_and(|user_id| settings.exempt_users.contains(&user_id))
    {
        return true;
    }

    // a bridge app is exempt along with every puppet it acts as, either because
    // the app itself or the puppeteer is exempt
    if identity
        .puppeteer_id
        .is_some_and(|user_id| settings.exempt_users.contains(&user_id))
    {
        return true;
    }

    identity
        .app_id
        .is_some_and(|app_id| settings.exempt_apps.contains(&app_id))
}

/// get the process-wide ratelimiter
fn global() -> &'static RateLimiter {
    GLOBAL.get_or_init(|| RateLimiter::new(settings()))
}

/// get the configured ratelimit settings, or the defaults if `init` was not called
fn settings() -> &'static ConfigRatelimit {
    SETTINGS.get_or_init(ConfigRatelimit::default)
}

fn keyed<K>(quota: Quota) -> KeyedLimiter<K>
where
    K: Clone + Hash + Eq,
{
    GovernorRateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>()
}

fn check_keyed<K>(
    limiter: &KeyedLimiter<K>,
    key: &K,
    bucket: Bucket,
    scope: Option<u64>,
    quota: Quota,
    ratelimit_scope: RatelimitScope,
) -> Decision
where
    K: Clone + Hash + Eq,
{
    let limit = quota.burst_size().get();
    match limiter.check_key(key) {
        Ok(snapshot) => Decision {
            bucket,
            scope,
            limit,
            remaining: snapshot.remaining_burst_capacity(),
            retry_after: None,
            ratelimit_scope,
        },
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            Decision {
                bucket,
                scope,
                limit,
                remaining: 0,
                retry_after: Some(wait),
                ratelimit_scope,
            }
        }
    }
}

fn charge_keyed<K>(limiter: &KeyedLimiter<K>, key: &K, extra: u32)
where
    K: Clone + Hash + Eq,
{
    for _ in 0..extra {
        let _ = limiter.check_key(key);
    }
}

fn client_ip(req: &Request<Body>) -> Option<IpAddr> {
    crate::util::headers::client_ip(req.headers()).or_else(|| {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
    })
}
