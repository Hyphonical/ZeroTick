use crate::config::{
	DEFAULT_CONNECT_TIMEOUT, SCAN_CONCURRENCY, SCAN_CONNECT_TIMEOUT, SCAN_RATE_LIMIT,
};
use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

fn styles() -> Styles {
	Styles::styled()
		.header(AnsiColor::Cyan.on_default().bold())
		.usage(AnsiColor::Cyan.on_default().bold())
		.literal(AnsiColor::White.on_default().bold())
		.placeholder(AnsiColor::Green.on_default())
		.valid(AnsiColor::Green.on_default())
		.invalid(AnsiColor::Red.on_default().bold())
		.error(AnsiColor::Red.on_default().bold())
}

#[derive(Parser)]
#[command(
    name = "zerotick",
    about = "Minecraft server reconnaissance tool",
    version,
    styles = styles(),
)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Command,

	/// Increase log verbosity (-v info, -vv debug, -vvv trace)
	#[arg(short, long, global = true, action = clap::ArgAction::Count)]
	pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Command {
	/// Query a Minecraft server
	Status {
		/// Server address (ip, ip:port, or domain)
		address: String,

		/// Connection timeout in milliseconds
		#[arg(short, long, default_value_t = DEFAULT_CONNECT_TIMEOUT.as_millis() as u64)]
		timeout: u64,
	},

	/// Scan an IP range for Minecraft servers
	Scan {
		/// CIDR range (e.g. 192.168.1.0/24)
		range: String,

		/// Target port
		#[arg(short, long, default_value = "25565")]
		port: u16,

		/// Maximum concurrent connections
		#[arg(short, long, default_value_t = SCAN_CONCURRENCY)]
		concurrency: usize,

		/// Connection timeout in milliseconds
		#[arg(short, long, default_value_t = SCAN_CONNECT_TIMEOUT.as_millis() as u64)]
		timeout: u64,

		/// Maximum new connections per second
		#[arg(long, default_value_t = SCAN_RATE_LIMIT)]
		rate: u32,
	},
}
