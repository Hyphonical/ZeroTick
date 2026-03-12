pub mod motd;
pub mod table;
pub mod tree;

use colored::{Color, ColoredString, Colorize};

// ── Palette ──────────────────────────────────────────

/// Soft blue — addresses, paths, server names.
pub const ACCENT: Color = Color::TrueColor {
	r: 135,
	g: 175,
	b: 215,
};

/// Soft teal — informational values.
pub const INFO: Color = Color::TrueColor {
	r: 175,
	g: 215,
	b: 215,
};

/// Gray — secondary, contextual text.
pub const MUTED: Color = Color::TrueColor {
	r: 128,
	g: 128,
	b: 128,
};

/// Dark gray — tree lines, table borders.
pub const SUBTLE: Color = Color::TrueColor {
	r: 88,
	g: 88,
	b: 88,
};

/// Very dark gray — unfilled progress bar. Applied via progress bar styling.
#[allow(dead_code)]
pub const SHADOW: Color = Color::TrueColor {
	r: 68,
	g: 68,
	b: 68,
};

// ── Symbols ──────────────────────────────────────────

pub const BULLET: &str = "●";
pub const CHECK: &str = "✓";
pub const CROSS: &str = "✗";
pub const WARN: &str = "⚠";
pub const DOT: &str = "·";

/// Spinner animation frames. Not yet used; reserved for animations during long operations.
#[allow(dead_code)]
pub const SPINNER_FRAMES: &[&str] = &["◐", "◓", "◑", "◒"];

// ── Helper Formatters ────────────────────────────────

/// Section header:  `● mc.hypixel.net`
pub fn header(text: &str) -> String {
	format!("{} {}", BULLET.cyan().bold(), text.color(ACCENT))
}

/// Success line:  `✓ message`
pub fn success(text: &str) -> String {
	format!("{} {}", CHECK.green(), text.color(ACCENT))
}

/// Error line:  `✗ message`
pub fn error(message: &str) -> String {
	format!("{} {}", CROSS.red().bold(), message.white())
}

/// Warning line:  `⚠ message`
pub fn warning(message: &str) -> String {
	format!("{} {}", WARN.yellow(), message.white())
}

/// Sub-detail:  `  · label  value`
pub fn detail(label: &str, value: &str) -> String {
	format!(
		"  {} {}  {}",
		DOT.dimmed(),
		label.dimmed(),
		value.color(INFO)
	)
}

/// Emphasized numeric value (white bold) with surrounding text.
pub fn value(v: impl std::fmt::Display) -> ColoredString {
	v.to_string().white().bold()
}

/// Muted contextual note (alternative to `.color(MUTED)`).
/// Provides a functional style for emphasis consistency across the codebase.
#[allow(dead_code)]
pub fn muted(text: &str) -> ColoredString {
	text.color(MUTED)
}
