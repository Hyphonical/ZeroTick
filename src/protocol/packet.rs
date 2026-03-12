use crate::config::MAX_PACKET_SIZE;
use crate::error::{Error, Result};
use crate::protocol::varint;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// ── Packet ───────────────────────────────────────────

/// A Minecraft protocol packet (ID + payload).
#[derive(Debug, Clone)]
pub struct Packet {
	pub id: i32,
	pub payload: Vec<u8>,
}

impl Packet {
	pub fn new(id: i32) -> Self {
		Self {
			id,
			payload: Vec::new(),
		}
	}

	// ── Builder methods (fluent) ─────────────────────

	pub fn write_varint(&mut self, value: i32) -> &mut Self {
		varint::encode(&mut self.payload, value);
		self
	}

	pub fn write_string(&mut self, s: &str) -> &mut Self {
		varint::encode(&mut self.payload, s.len() as i32);
		self.payload.extend_from_slice(s.as_bytes());
		self
	}

	pub fn write_u16(&mut self, value: u16) -> &mut Self {
		self.payload.extend_from_slice(&value.to_be_bytes());
		self
	}

	pub fn write_i64(&mut self, value: i64) -> &mut Self {
		self.payload.extend_from_slice(&value.to_be_bytes());
		self
	}

	/// Append raw bytes to the payload.
	/// Not yet used; reserved for complex packet structures with binary data.
	#[allow(dead_code)]
	pub fn write_bytes(&mut self, data: &[u8]) -> &mut Self {
		self.payload.extend_from_slice(data);
		self
	}

	// ── Wire format ──────────────────────────────────

	/// Encode into a length-prefixed byte buffer ready for the wire.
	pub fn encode(&self) -> Vec<u8> {
		let id_len = varint::size(self.id);
		let data_len = id_len + self.payload.len();
		let total = varint::size(data_len as i32) + data_len;

		let mut buf = Vec::with_capacity(total);
		varint::encode(&mut buf, data_len as i32);
		varint::encode(&mut buf, self.id);
		buf.extend_from_slice(&self.payload);
		buf
	}

	/// Encode and send this packet over an async writer.
	pub async fn send<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
		writer.write_all(&self.encode()).await?;
		writer.flush().await?;
		Ok(())
	}

	/// Read one length-prefixed packet from an async reader.
	pub async fn recv<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
		let length = varint::read(reader).await? as usize;

		if length == 0 {
			return Err(Error::protocol("empty packet"));
		}
		if length > MAX_PACKET_SIZE {
			return Err(Error::PacketTooLarge(length));
		}

		let mut data = vec![0u8; length];
		reader.read_exact(&mut data).await?;

		let (id, consumed) = varint::decode(&data)?;
		let payload = data[consumed..].to_vec();

		Ok(Self { id, payload })
	}

	/// Create a reader for parsing fields from this packet's payload.
	pub fn reader(&self) -> PacketReader<'_> {
		PacketReader::new(&self.payload)
	}
}

// ── Payload Reader ───────────────────────────────────

/// Zero-copy reader for extracting typed fields from a packet payload.
pub struct PacketReader<'a> {
	data: &'a [u8],
	pos: usize,
}

impl<'a> PacketReader<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self { data, pos: 0 }
	}

	pub fn read_varint(&mut self) -> Result<i32> {
		let (value, consumed) = varint::decode(&self.data[self.pos..])?;
		self.pos += consumed;
		Ok(value)
	}

	pub fn read_string(&mut self) -> Result<&'a str> {
		let len = self.read_varint()? as usize;
		if self.pos + len > self.data.len() {
			return Err(Error::protocol("string extends beyond packet"));
		}
		let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
			.map_err(|e| Error::protocol(e.to_string()))?;
		self.pos += len;
		Ok(s)
	}

	/// Read a signed 64-bit integer. Not yet used; reserved for extended protocol support.
	#[allow(dead_code)]
	pub fn read_i64(&mut self) -> Result<i64> {
		if self.pos + 8 > self.data.len() {
			return Err(Error::protocol("not enough data for i64"));
		}
		let bytes: [u8; 8] = self.data[self.pos..self.pos + 8]
			.try_into()
			.map_err(|_| Error::protocol("i64 read failed"))?;
		self.pos += 8;
		Ok(i64::from_be_bytes(bytes))
	}

	/// Return the remaining unread bytes. Not yet used; reserved for packet introspection.
	#[allow(dead_code)]
	pub fn remaining(&self) -> &'a [u8] {
		&self.data[self.pos..]
	}
}
