pub mod bridge;
pub mod platform;
pub mod portal;
pub mod realm;

// example linking flow
// 1. discord user does /link -> PortalCreate + PortalLink commands
// x. either bridge or discord checks if already linked?
// 2. bridge sends PortalRequested event to target platform (lamprey)
// 3. lamprey sends confirmation message to channel, waits for reply
// 4. sends PortalLinkAccept(?) command to bridge
// 5. bridge saves portal in database, maybe emits event
