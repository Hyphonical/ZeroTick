use std::time::Duration;

// ── Protocol ─────────────────────────────────────────

/// Minecraft 1.21.4
pub const PROTOCOL_VERSION: i32 = 769;

pub const DEFAULT_PORT: u16 = 25565;

// ── Timeouts ─────────────────────────────────────────

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Not yet used; reserved for future socket read timeout support.
#[allow(dead_code)]
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(5);

pub const SCAN_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// Not yet used; reserved for future socket read timeout support.
#[allow(dead_code)]
pub const SCAN_READ_TIMEOUT: Duration = Duration::from_secs(3);
pub const SCAN_CONCURRENCY: usize = 256;
pub const SCAN_RATE_LIMIT: u32 = 1000;

// ── Protocol Limits ──────────────────────────────────

/// Vanilla server maximum packet size.
pub const MAX_PACKET_SIZE: usize = 2 * 1024 * 1024;

// ── Mojang API ───────────────────────────────────────

pub const MOJANG_SESSION_API: &str = "https://sessionserver.mojang.com";
pub const MOJANG_BLOCKED_SERVERS: &str = "https://sessionserver.mojang.com/blockedservers";

/// Max player profiles to look up per status query (avoid hammering Mojang).
pub const MAX_PROFILE_LOOKUPS: usize = 5;
