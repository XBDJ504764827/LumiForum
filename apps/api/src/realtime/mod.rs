mod bus;
mod hub;
mod presence;
mod protocol;

pub use bus::RealtimeBus;
pub use hub::RealtimeHub;
pub use presence::{PresenceService, PresenceStatus};
pub use protocol::{ClientMessage, RealtimeEnvelope, ServerMessage};

pub const USER_EVENTS_CHANNEL: &str = "realtime:user-events";
pub const DEFAULT_MAX_CONNECTIONS_PER_USER: usize = 5;
pub const DEFAULT_HEARTBEAT_SECS: u64 = 30;
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 90;
pub const DEFAULT_PRESENCE_TTL_SECS: u64 = 60;
pub const DEFAULT_CONNECT_RATE_LIMIT: u64 = 30;
