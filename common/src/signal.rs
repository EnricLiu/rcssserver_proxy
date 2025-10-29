use bytes::Bytes;
use tungstenite::Message;

#[derive(Clone, Debug)]
pub enum Signal {
    Data(Bytes),
    Ping(u64),
    Pong(u64),
    Unknown,
}

impl From<Message> for Signal {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Binary(bytes) => Signal::Data(bytes),
            Message::Ping(bytes) => Signal::Ping(bytes_to_u64(bytes)),
            Message::Pong(bytes) => Signal::Pong(bytes_to_u64(bytes)),
            _ => Signal::Unknown,
        }
    }
}

impl From<Signal> for Message {
    fn from(signal: Signal) -> Self {
        match signal {
            Signal::Data(bytes) => Message::Binary(bytes),
            Signal::Ping(num) => Message::Ping(u64_to_bytes(num)),
            Signal::Pong(num) => Message::Pong(u64_to_bytes(num)),
            Signal::Unknown => Message::Text("Unknown signal".to_string().into()),
        }
    }
}

impl Signal {
    pub fn data(bytes: impl Into<Bytes>) -> Self {
        Signal::Data(bytes.into())
    }
}

pub fn bytes_to_u64(bytes: Bytes) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes);
    u64::from_be_bytes(buf)
}

pub fn u64_to_bytes(num: u64) -> Bytes {
    let buf: Box<[u8]> = num.to_be_bytes().into();
    Bytes::from(buf)
}