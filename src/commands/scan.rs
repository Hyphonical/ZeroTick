use crate::error::Result;
use crate::scanner::probe::{self, ProbeResult};
use crate::scanner::range;
use crate::style;
use crate::style::table::Table;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

pub async fn run(
	cidr: &str,
	port: u16,
	concurrency: usize,
	timeout_ms: u64,
	rate: u32,
) -> Result<()> {
	let addrs = range::parse_cidr(cidr)?;
	let total = range::cidr_size(cidr).unwrap_or(addrs.len());

	let connect_timeout = Duration::from_millis(timeout_ms);
	let read_timeout = Duration::from_millis(timeout_ms);

	// Header
	println!(
		"{}\n{}\n{}",
		style::header(&format!("Scan: {cidr}:{port}  ({total} addresses)")),
		style::detail("Rate", &format!("{rate}/s")),
		style::detail("Timeout", &format!("{timeout_ms}ms")),
	);
	println!();

	// Progress bar
	let pb = ProgressBar::new(total as u64);
	pb.set_style(
		ProgressStyle::with_template(
			"  {bar:40.cyan/238} {pos:>6}/{len} {elapsed_precise} elapsed  {msg}",
		)
		.unwrap()
		.progress_chars("━━╌"),
	);

	let semaphore = Arc::new(Semaphore::new(concurrency));
	let results: Arc<Mutex<Vec<ProbeResult>>> = Arc::new(Mutex::new(Vec::new()));

	let interval = if rate > 0 {
		Duration::from_secs_f64(1.0 / rate as f64)
	} else {
		Duration::ZERO
	};

	let mut handles = Vec::with_capacity(total);

	for addr in addrs {
		let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
		let results = Arc::clone(&results);
		let pb = pb.clone();

		handles.push(tokio::spawn(async move {
			let result = probe::probe(addr, port, connect_timeout, read_timeout).await;

			if let Some(r) = result {
				let mut lock = results.lock().await;
				lock.push(r);
				pb.set_message(format!("{} found", lock.len()).green().to_string());
			}

			pb.inc(1);
			drop(permit);
		}));

		if !interval.is_zero() {
			sleep(interval).await;
		}
	}

	for handle in handles {
		let _ = handle.await;
	}

	pb.finish_and_clear();

	// ── Results ──────────────────────────────────────
	let final_results: Vec<ProbeResult> = {
		let lock = results.lock().await;
		lock.clone()
	};

	println!();

	if final_results.is_empty() {
		println!("{}", style::warning("no servers found"));
		return Ok(());
	}

	println!(
		"{}",
		style::success(&format!("{} servers found", final_results.len()))
	);
	println!();

	let mut table = Table::new(vec!["Address", "Version", "Players", "Ping"]);

	let mut sorted = final_results;
	sorted.sort_by_key(|r| r.latency);

	for r in &sorted {
		table.add_row(vec![
			format!("{}:{}", r.addr, r.port)
				.color(style::ACCENT)
				.to_string(),
			r.status.version.name.color(style::INFO).to_string(),
			format!("{}/{}", r.status.players.online, r.status.players.max),
			format!("{}ms", r.latency.as_millis()),
		]);
	}

	print!("{}", table.render());

	Ok(())
}
