use crate::error::{Error, Result};
use rand::seq::SliceRandom;
use std::net::Ipv4Addr;

/// Parse a CIDR notation string and return a randomized list of addresses.
pub fn parse_cidr(input: &str) -> Result<Vec<Ipv4Addr>> {
	let (addr_str, prefix_str) = input.split_once('/').ok_or_else(|| {
		Error::InvalidAddress("expected CIDR notation (e.g. 192.168.1.0/24)".into())
	})?;

	let base: Ipv4Addr = addr_str
		.parse()
		.map_err(|_| Error::InvalidAddress(format!("invalid IP: {addr_str}")))?;

	let prefix: u32 = prefix_str
		.parse()
		.map_err(|_| Error::InvalidAddress(format!("invalid prefix: {prefix_str}")))?;

	if prefix > 32 {
		return Err(Error::InvalidAddress(format!(
			"prefix too large: /{prefix}"
		)));
	}

	let mask: u32 = if prefix == 0 {
		0
	} else {
		!0u32 << (32 - prefix)
	};

	let base_u32 = u32::from(base);
	let network = base_u32 & mask;
	let broadcast = network | !mask;
	let count = (broadcast - network + 1) as usize;

	if count > 16_777_216 {
		return Err(Error::InvalidAddress(
			"range too large (max /8 = 16M addresses)".into(),
		));
	}

	let mut addrs: Vec<Ipv4Addr> = (network..=broadcast).map(Ipv4Addr::from).collect();

	let mut rng = rand::rng();
	addrs.shuffle(&mut rng);

	Ok(addrs)
}

/// Return the total number of addresses in a CIDR range (for display).
pub fn cidr_size(input: &str) -> Option<usize> {
	let (_addr_str, prefix_str) = input.split_once('/')?;
	let prefix: u32 = prefix_str.parse().ok()?;
	if prefix > 32 {
		return None;
	}
	Some(1usize << (32 - prefix))
}
