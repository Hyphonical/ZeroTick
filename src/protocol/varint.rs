use crate::error::{Error, Result};
use tokio::io::{AsyncRead, AsyncReadExt};

const MAX_BYTES: usize = 5;

// ── Encode ───────────────────────────────────────────

/// Append a VarInt to a byte buffer.
pub fn encode(buf: &mut Vec<u8>, value: i32) {
	let mut v = value as u32;
	loop {
		let mut byte = (v & 0x7F) as u8;
		v >>= 7;
		if v != 0 {
			byte |= 0x80;
		}
		buf.push(byte);
		if v == 0 {
			break;
		}
	}
}

/// Number of bytes needed to encode this value.
pub fn size(value: i32) -> usize {
	let mut v = value as u32;
	let mut len = 0;
	loop {
		len += 1;
		v >>= 7;
		if v == 0 {
			return len;
		}
	}
}

// ── Decode (from slice, sync) ────────────────────────

/// Decode a VarInt from a byte slice.
/// Returns `(value, bytes_consumed)`.
pub fn decode(data: &[u8]) -> Result<(i32, usize)> {
	let mut result: i32 = 0;
	let mut shift: u32 = 0;

	for (i, &byte) in data.iter().enumerate().take(MAX_BYTES) {
		result |= ((byte & 0x7F) as i32) << shift;
		if byte & 0x80 == 0 {
			return Ok((result, i + 1));
		}
		shift += 7;
	}

	Err(Error::VarIntTooLong)
}

// ── Decode (from async reader) ───────────────────────

/// Read a VarInt from an async byte stream.
pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32> {
	let mut result: i32 = 0;
	let mut shift: u32 = 0;

	for _ in 0..MAX_BYTES {
		let byte = reader.read_u8().await?;
		result |= ((byte & 0x7F) as i32) << shift;
		if byte & 0x80 == 0 {
			return Ok(result);
		}
		shift += 7;
	}

	Err(Error::VarIntTooLong)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn round_trip() {
		let values = [0, 1, 127, 128, 255, 25565, 2097151, -1, i32::MAX, i32::MIN];
		for &v in &values {
			let mut buf = Vec::new();
			encode(&mut buf, v);
			assert_eq!(buf.len(), size(v));
			let (decoded, consumed) = decode(&buf).unwrap();
			assert_eq!(decoded, v);
			assert_eq!(consumed, buf.len());
		}
	}

	#[test]
	fn too_long() {
		let data = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80]; // 6 continuation bytes
		assert!(decode(&data).is_err());
	}
}
