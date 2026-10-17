use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, RwLock, mpsc};

use super::Error;
use super::blob::{self, TAG_SIZE, Writer};

const SOCKET: &str = "/var/run/ubus/ubus.sock";
const TIMEOUT: Duration = Duration::from_secs(15);
const HEADER_SIZE: usize = 8;

const MSG_STATUS: u8 = 1;
const MSG_DATA: u8 = 2;
const MSG_LOOKUP: u8 = 4;
const MSG_INVOKE: u8 = 5;

const ATTR_STATUS: u32 = 1;
const ATTR_OBJPATH: u32 = 2;
const ATTR_OBJID: u32 = 3;
const ATTR_METHOD: u32 = 4;
const ATTR_DATA: u32 = 7;

struct Message {
	kind: u8,
	body: Vec<u8>,
}

enum Reply {
	Data(Vec<u8>),
	Status(u32),
}

type Pending = Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<Reply>>>>;

struct Connection {
	writer: Mutex<OwnedWriteHalf>,
	pending: Pending,
	alive: Arc<AtomicBool>,
	seq: AtomicU16,
}

impl Connection {
	async fn open() -> Result<Self, Error> {
		let stream = UnixStream::connect(SOCKET).await?;
		let (mut reader, writer) = stream.into_split();
		read_message(&mut reader).await?;
		let pending = Pending::default();
		let alive = Arc::new(AtomicBool::new(true));
		tokio::spawn(dispatch(reader, pending.clone(), alive.clone()));
		Ok(Self {
			writer: Mutex::new(writer),
			pending,
			alive,
			seq: AtomicU16::new(1),
		})
	}

	async fn request(&self, kind: u8, peer: u32, fill: impl FnOnce(&mut Writer)) -> Result<Vec<Vec<u8>>, Error> {
		let seq = self.seq.fetch_add(1, Ordering::Relaxed);
		let (tx, mut rx) = mpsc::unbounded_channel();
		self.pending.lock().await.insert(seq, tx);

		let mut writer = Writer::default();
		writer.nested(0, fill);
		let body = writer.into_inner();
		let mut frame = Vec::with_capacity(HEADER_SIZE + body.len());
		frame.extend_from_slice(&[0, kind]);
		frame.extend_from_slice(&seq.to_be_bytes());
		frame.extend_from_slice(&peer.to_be_bytes());
		frame.extend_from_slice(&body);

		let result = async {
			self.writer.lock().await.write_all(&frame).await?;
			let mut data = Vec::new();
			loop {
				match rx.recv().await.ok_or(Error::Disconnected)? {
					Reply::Data(chunk) => data.push(chunk),
					Reply::Status(0) => return Ok(data),
					Reply::Status(code) => return Err(Error::Status(code)),
				}
			}
		};
		let result = tokio::time::timeout(TIMEOUT, result).await.unwrap_or(Err(Error::Timeout));
		self.pending.lock().await.remove(&seq);
		result
	}
}

async fn read_message(reader: &mut OwnedReadHalf) -> Result<(u16, Message), Error> {
	let mut head = [0u8; HEADER_SIZE + TAG_SIZE];
	reader.read_exact(&mut head).await?;
	let seq = u16::from_be_bytes([head[2], head[3]]);
	let len = blob::tag_len(head[HEADER_SIZE..].try_into().unwrap_or_default());
	let mut body = vec![0u8; len.saturating_sub(TAG_SIZE)];
	reader.read_exact(&mut body).await?;
	Ok((seq, Message { kind: head[1], body }))
}

async fn dispatch(mut reader: OwnedReadHalf, pending: Pending, alive: Arc<AtomicBool>) {
	while let Ok((seq, message)) = read_message(&mut reader).await {
		let reply = match message.kind {
			MSG_DATA => Some(Reply::Data(message.body)),
			MSG_STATUS => blob::attrs(&message.body)
				.find(|attr| attr.id == ATTR_STATUS)
				.and_then(|attr| attr.u32())
				.map(Reply::Status),
			_ => None,
		};
		if let Some(reply) = reply
			&& let Some(tx) = pending.lock().await.get(&seq)
		{
			let _ = tx.send(reply);
		}
	}
	alive.store(false, Ordering::Relaxed);
	pending.lock().await.clear();
}

#[derive(Clone, Default)]
pub struct Ubus {
	connection: Arc<RwLock<Option<Arc<Connection>>>>,
	objects: Arc<RwLock<HashMap<String, u32>>>,
}

impl Ubus {
	async fn connection(&self) -> Result<Arc<Connection>, Error> {
		if let Some(conn) = self
			.connection
			.read()
			.await
			.as_ref()
			.filter(|conn| conn.alive.load(Ordering::Relaxed))
		{
			return Ok(conn.clone());
		}
		let mut slot = self.connection.write().await;
		if let Some(conn) = slot.as_ref().filter(|conn| conn.alive.load(Ordering::Relaxed)) {
			return Ok(conn.clone());
		}
		let conn = Arc::new(Connection::open().await?);
		*slot = Some(conn.clone());
		self.objects.write().await.clear();
		Ok(conn)
	}

	async fn lookup(&self, conn: &Connection, path: &str) -> Result<u32, Error> {
		if let Some(id) = self.objects.read().await.get(path) {
			return Ok(*id);
		}
		let replies = conn.request(MSG_LOOKUP, 0, |w| w.str(ATTR_OBJPATH, path)).await?;
		let id = replies
			.first()
			.and_then(|data| {
				blob::attrs(data)
					.find(|attr| attr.id == ATTR_OBJID)
					.and_then(|attr| attr.u32())
			})
			.ok_or(Error::Status(super::STATUS_NOT_FOUND))?;
		self.objects.write().await.insert(path.to_owned(), id);
		Ok(id)
	}

	pub async fn call(&self, path: &str, method: &str, args: Value) -> Result<Value, Error> {
		let conn = self.connection().await?;
		let id = self.lookup(&conn, path).await?;
		let args = match args {
			Value::Object(map) => map,
			_ => Map::new(),
		};
		let replies = conn
			.request(MSG_INVOKE, id, |w| {
				w.u32(ATTR_OBJID, id);
				w.str(ATTR_METHOD, method);
				w.nested(ATTR_DATA, |w| w.table(&args));
			})
			.await;
		if let Err(Error::Status(super::STATUS_NOT_FOUND)) = replies {
			self.objects.write().await.remove(path);
		}
		let mut result = Map::new();
		for reply in replies? {
			if let Some(data) = blob::attrs(&reply).find(|attr| attr.id == ATTR_DATA) {
				result.extend(blob::decode_table(data.data));
			}
		}
		Ok(Value::Object(result))
	}

	pub async fn call_as<T: DeserializeOwned>(&self, path: &str, method: &str, args: Value) -> Result<T, Error> {
		Ok(serde_json::from_value(self.call(path, method, args).await?)?)
	}

	pub async fn objects(&self, prefix: &str) -> Result<Vec<String>, Error> {
		let conn = self.connection().await?;
		let pattern = format!("{prefix}*");
		let replies = conn.request(MSG_LOOKUP, 0, |w| w.str(ATTR_OBJPATH, &pattern)).await?;
		Ok(replies
			.iter()
			.filter_map(|data| blob::attrs(data).find(|attr| attr.id == ATTR_OBJPATH))
			.map(|attr| String::from_utf8_lossy(attr.data.strip_suffix(&[0]).unwrap_or(attr.data)).into_owned())
			.collect())
	}
}
