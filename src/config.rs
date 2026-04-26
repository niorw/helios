//! Global configuration constants for Helios.
//!
//! Centralizes all hard-coded values (app identity, paths, defaults)
//! so they can be changed in one place.

// ─── Application Identity ────────────────────────────────────────────

pub const APP_NAME: &str = "helios";
pub const APP_NAME_DISPLAY: &str = "Helios";
pub const APP_VERSION: &str = "1.0.0";
pub const APP_ABOUT: &str = "Helios - A blazing-fast terminal API client";

// ─── Data Directory (directories crate) ──────────────────────────────

/// Used with `directories::ProjectDirs::from(qualifier, organization, application)`.
pub const APP_QUALIFIER: &str = "com";
pub const APP_ORGANIZATION: &str = "helios";
pub const APP_APPLICATION: &str = "helios";

pub const DATA_FILE_NAME: &str = "data.json";

// ─── Installation ────────────────────────────────────────────────────

pub const INSTALL_DIR: &str = "/usr/local/bin";
pub const BINARY_NAME: &str = APP_NAME;

// ─── Default Values ──────────────────────────────────────────────────

pub const DEFAULT_URL: &str = "https://httpbin.org/get";
pub const DEFAULT_USER_AGENT: &str = "Helios/1.0";

// ─── TUI Theme ───────────────────────────────────────────────────────

pub const BG_HEX: &str = "#14141E";
pub const SURFACE_HEX: &str = "#1E1E2D";
pub const ACCENT_HEX: &str = "#B48CFF";
pub const BORDER_HEX: &str = "#8C64FF";
pub const TEXT_HEX: &str = "#DCDCF0";
pub const TEXT_DIM_HEX: &str = "#6C6C8A";
pub const SUCCESS_HEX: &str = "#50FA7B";
pub const WARN_HEX: &str = "#FFB86C";
pub const ERROR_HEX: &str = "#FF5555";

// ─── Timing ──────────────────────────────────────────────────────────

/// Window prefix timeout in ticks (~50ms per tick → 120 ≈ 6s).
pub const WINDOW_PREFIX_TIMEOUT_TICKS: u64 = 120;

/// Status message expiry in ticks.
pub const STATUS_MESSAGE_TIMEOUT_TICKS: u64 = 120;
