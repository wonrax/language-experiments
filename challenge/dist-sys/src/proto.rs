use std::{
    io::{self, Write},
    sync::{Arc, Mutex, OnceLock},
};

use serde_json::{Map, Value};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Message {
    pub src: String,
    pub dest: String,
    pub body: Body,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Body {
    #[serde(rename = "type")]
    pub typ: String,
    pub msg_id: Option<u64>,
    pub in_reply_to: Option<u64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug)]
pub struct Request {
    pub typ: String,
    pub src: Option<String>,
    pub dest: Option<String>,
    pub body: Option<Map<String, Value>>,
}

pub type Response = Request;

impl From<anyhow::Error> for Message {
    fn from(value: anyhow::Error) -> Self {
        Message {
            src: "TODO".into(),
            dest: "TODO".into(),
            body: Body {
                typ: "error".into(),
                msg_id: None,
                in_reply_to: None,
                extra: Map::from_iter([("error".to_string(), Value::String(value.to_string()))]),
            },
        }
    }
}

// RPC protocol
#[derive(Debug)]
pub struct RPC {
    // A lock to ensure only one task is writing to stdout at a time
    write_lock: Arc<Mutex<()>>,

    // global incrementing counter for message ids
    msg_id: u64,
}

impl RPC {
    pub fn new() -> Self {
        Self {
            write_lock: Arc::new(Mutex::new(())),
            msg_id: 0,
        }
    }

    pub fn send(&mut self, mut message: Message) {
        self.msg_id += 1;
        message.body.msg_id = Some(self.msg_id);
        let message_str = serde_json::to_string(&message).unwrap() + "\n";
        let _lock = self.write_lock.lock().unwrap();
        io::stdout().write_all(message_str.as_bytes()).unwrap();
    }
}

pub static mut RPC_INSTANCE: OnceLock<RPC> = OnceLock::new();
