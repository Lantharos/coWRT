use serde_json::{Map, Number, Value};

const EXTENDED: u32 = 1 << 31;
const ID_SHIFT: u32 = 24;
const ID_MASK: u32 = 0x7f;
const LEN_MASK: u32 = 0x00ff_ffff;

const TYPE_ARRAY: u32 = 1;
const TYPE_TABLE: u32 = 2;
const TYPE_STRING: u32 = 3;
const TYPE_INT64: u32 = 4;
const TYPE_INT32: u32 = 5;
const TYPE_INT16: u32 = 6;
const TYPE_INT8: u32 = 7;
const TYPE_DOUBLE: u32 = 8;

pub const TAG_SIZE: usize = 4;

const fn align(len: usize) -> usize {
	(len + 3) & !3
}

pub fn tag_len(tag: [u8; TAG_SIZE]) -> usize {
	(u32::from_be_bytes(tag) & LEN_MASK) as usize
}

pub struct Attr<'a> {
	pub id: u32,
	pub data: &'a [u8],
}

impl Attr<'_> {
	pub fn u32(&self) -> Option<u32> {
		Some(u32::from_be_bytes(self.data.get(..4)?.try_into().ok()?))
	}
}

pub fn attrs(mut buf: &[u8]) -> impl Iterator<Item = Attr<'_>> {
	std::iter::from_fn(move || {
		let tag = u32::from_be_bytes(buf.get(..TAG_SIZE)?.try_into().ok()?);
		let len = (tag & LEN_MASK) as usize;
		if len < TAG_SIZE || len > buf.len() {
			return None;
		}
		let attr = Attr {
			id: (tag >> ID_SHIFT) & ID_MASK,
			data: &buf[TAG_SIZE..len],
		};
		buf = &buf[align(len).min(buf.len())..];
		Some(attr)
	})
}

#[derive(Default)]
pub struct Writer {
	buf: Vec<u8>,
}

impl Writer {
	pub fn into_inner(self) -> Vec<u8> {
		self.buf
	}

	fn open(&mut self) -> usize {
		let start = self.buf.len();
		self.buf.extend_from_slice(&[0; TAG_SIZE]);
		start
	}

	fn close(&mut self, start: usize, id: u32, extended: bool) {
		let len = (self.buf.len() - start) as u32;
		let tag = len | (id & ID_MASK) << ID_SHIFT | if extended { EXTENDED } else { 0 };
		self.buf[start..start + TAG_SIZE].copy_from_slice(&tag.to_be_bytes());
		self.pad();
	}

	fn pad(&mut self) {
		self.buf.resize(align(self.buf.len()), 0);
	}

	pub fn nested(&mut self, id: u32, fill: impl FnOnce(&mut Self)) {
		let start = self.open();
		fill(self);
		self.close(start, id, false);
	}

	pub fn bytes(&mut self, id: u32, data: &[u8]) {
		let start = self.open();
		self.buf.extend_from_slice(data);
		self.close(start, id, false);
	}

	pub fn u32(&mut self, id: u32, value: u32) {
		self.bytes(id, &value.to_be_bytes());
	}

	pub fn str(&mut self, id: u32, value: &str) {
		let start = self.open();
		self.buf.extend_from_slice(value.as_bytes());
		self.buf.push(0);
		self.close(start, id, false);
	}

	pub fn table(&mut self, table: &Map<String, Value>) {
		for (name, value) in table {
			self.value(name, value);
		}
	}

	fn value(&mut self, name: &str, value: &Value) {
		let kind = match value {
			Value::Null => return,
			Value::Bool(_) => TYPE_INT8,
			Value::Number(n) if n.is_f64() => TYPE_DOUBLE,
			Value::Number(n) if n.as_i64().is_some_and(|n| i32::try_from(n).is_ok()) => TYPE_INT32,
			Value::Number(_) => TYPE_INT64,
			Value::String(_) => TYPE_STRING,
			Value::Array(_) => TYPE_ARRAY,
			Value::Object(_) => TYPE_TABLE,
		};
		let start = self.open();
		self.buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
		self.buf.extend_from_slice(name.as_bytes());
		self.buf.push(0);
		self.pad();
		match value {
			Value::Bool(b) => self.buf.push(u8::from(*b)),
			Value::Number(n) if kind == TYPE_DOUBLE => self.buf.extend_from_slice(&n.as_f64().unwrap_or_default().to_be_bytes()),
			Value::Number(n) if kind == TYPE_INT32 => self
				.buf
				.extend_from_slice(&(n.as_i64().unwrap_or_default() as i32).to_be_bytes()),
			Value::Number(n) => self
				.buf
				.extend_from_slice(&n.as_i64().unwrap_or(n.as_u64().unwrap_or_default() as i64).to_be_bytes()),
			Value::String(s) => {
				self.buf.extend_from_slice(s.as_bytes());
				self.buf.push(0);
			}
			Value::Array(items) => items.iter().for_each(|item| self.value("", item)),
			Value::Object(table) => self.table(table),
			Value::Null => unreachable!(),
		}
		self.close(start, kind, true);
	}
}

pub fn decode_table(buf: &[u8]) -> Map<String, Value> {
	attrs(buf).filter_map(|attr| decode_entry(&attr)).collect()
}

fn decode_array(buf: &[u8]) -> Vec<Value> {
	attrs(buf)
		.filter_map(|attr| decode_entry(&attr).map(|(_, value)| value))
		.collect()
}

fn decode_entry(attr: &Attr) -> Option<(String, Value)> {
	let name_len = u16::from_be_bytes(attr.data.get(..2)?.try_into().ok()?) as usize;
	let name = std::str::from_utf8(attr.data.get(2..2 + name_len)?).ok()?.to_owned();
	let payload = attr.data.get(align(2 + name_len + 1)..)?;
	let value = match attr.id {
		TYPE_TABLE => Value::Object(decode_table(payload)),
		TYPE_ARRAY => Value::Array(decode_array(payload)),
		TYPE_STRING => Value::String(String::from_utf8_lossy(payload.strip_suffix(&[0]).unwrap_or(payload)).into_owned()),
		TYPE_INT64 => Value::from(i64::from_be_bytes(payload.get(..8)?.try_into().ok()?)),
		TYPE_INT32 => Value::from(i32::from_be_bytes(payload.get(..4)?.try_into().ok()?)),
		TYPE_INT16 => Value::from(i16::from_be_bytes(payload.get(..2)?.try_into().ok()?)),
		TYPE_INT8 => Value::Bool(*payload.first()? != 0),
		TYPE_DOUBLE => {
			Number::from_f64(f64::from_be_bytes(payload.get(..8)?.try_into().ok()?)).map_or(Value::Null, Value::Number)
		}
		_ => Value::Null,
	};
	Some((name, value))
}
