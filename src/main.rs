mod cli;
mod commands;
mod config;
mod error;
mod mojang;
mod net;
mod protocol;
mod scanner;
mod style;
mod types;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
	// Install the rustls ring crypto provider before anything touches TLS.
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("failed to install rustls crypto provider");

	let cli = cli::Cli::parse();

	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
			let level = match cli.verbose {
				0 => "error",
				1 => "zerotick=info",
				2 => "zerotick=debug",
				_ => "trace",
			};
			EnvFilter::new(level)
		}))
		.with_writer(std::io::stderr)
		.init();

	let result = match cli.command {
		cli::Command::Status { address, timeout } => commands::status::run(&address, timeout).await,
		cli::Command::Serve { host, port } => commands::serve::run(&host, port).await,
		cli::Command::Scan {
			range,
			port,
			concurrency,
			timeout,
			rate,
		} => commands::scan::run(&range, port, concurrency, timeout, rate).await,
	};

	if let Err(err) = result {
		eprintln!("{}", style::error(&err.to_string()));
		std::process::exit(1);
	}
}
