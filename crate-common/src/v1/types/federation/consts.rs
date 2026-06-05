use std::time::Duration;

// NOTE: maybe i should put the below into some ProtocolConfig struct?

/// how long to keep expired keys before deleting them
pub const EXPIRED_KEY_RETENTION: Duration = Duration::from_secs(24 * 3600);

/// how long to keep keys alive
pub const KEY_EXPIRY: Duration = Duration::from_secs(3600);

/// create a new key if the freshest one expires within this window
pub const KEY_ROTATION_WINDOW: Duration = Duration::from_secs(300);

/// how long a signed request is valid for
pub const SIGNATURE_MAX_AGE: Duration = Duration::from_secs(30);
