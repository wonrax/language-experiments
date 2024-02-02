use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex, OnceLock},
};

use async_runtime::runtime::current;
use futures::{Future, StreamExt};
use serde_json::{Map, Value};

use crate::{client::RequestFuture, stdin::StdinListener};

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<u64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub typ: String,
    pub src: Option<String>,
    pub dest: Option<String>,
    pub body: Option<Map<String, Value>>,
}

pub type Response = Request;

impl Response {
    pub fn new(typ: &str) -> Self {
        Self {
            typ: typ.into(),
            src: None,
            dest: None,
            body: None,
        }
    }

    pub fn with_body(mut self, body: Map<String, Value>) -> Self {
        self.body = Some(body);
        self
    }
}

// RPC protocol
#[derive(Debug)]
pub struct Protocol {
    // A lock to ensure only one task is writing to stdout at a time
    write_lock: Arc<Mutex<()>>,

    // global incrementing counter for message ids
    msg_id: u64,

    // map of message ids to channels to send responses on
    responses: HashMap<u64, RequestFuture>,
}

impl Protocol {
    pub fn new() -> Self {
        Self {
            write_lock: Arc::new(Mutex::new(())),
            msg_id: 0,
            responses: HashMap::new(),
        }
    }

    // fire and forget
    // TODO spawn a task so that the write is queued instead of blocking
    pub fn send(&mut self, mut message: Message) {
        self.msg_id += 1;
        message.body.msg_id = Some(self.msg_id);
        let message_str = serde_json::to_string(&message).unwrap() + "\n";
        let _lock = self.write_lock.lock().unwrap();
        io::stdout().write_all(message_str.as_bytes()).unwrap();
    }

    // send and wait for response
    // TODO support timeout
    pub async fn call(
        &mut self,
        mut message: Message,
        fut: RequestFuture,
    ) -> Result<Response, anyhow::Error> {
        // let (send, recv) = crossbeam_channel::bounded(1);

        self.msg_id += 1;
        message.body.msg_id = Some(self.msg_id);

        self.responses
            .insert(message.body.msg_id.unwrap(), fut.clone());

        let message_str = serde_json::to_string(&message).unwrap() + "\n";
        let _lock = self.write_lock.lock().unwrap();
        io::stdout().write_all(message_str.as_bytes()).unwrap();
        drop(_lock);

        Ok(fut.await)
    }

    // start accepting messages
    pub async fn listen<F, R>(&mut self, handler: F)
    where
        F: FnMut(Request) -> R + Send + Clone + 'static,
        R: Future<Output = Response> + Send,
    {
        let mut stdin_listener = StdinListener::new();

        loop {
            let message: Message =
                serde_json::from_str(&stdin_listener.next().await.unwrap()).unwrap();

            // TODO find optimization opportunity here (e.g. spawn task)
            if self
                .responses
                .contains_key(&message.body.in_reply_to.unwrap_or(0))
            {
                self.responses
                    .remove(&message.body.in_reply_to.unwrap_or(0))
                    .unwrap()
                    .set_response(Response {
                        typ: message.body.typ,
                        src: Some(message.src),
                        dest: Some(message.dest),
                        body: Some(message.body.extra),
                    });

                continue;
            }

            let mut cloned_handler = handler.clone();

            // If the message is not a response, it must be a request
            current().spawn(async move {
                let response = cloned_handler(Request {
                    typ: message.body.typ,
                    src: Some(message.src.clone()),
                    dest: Some(message.dest.clone()),
                    body: Some(message.body.extra),
                })
                .await;

                unsafe {
                    PROTOCOL.get_mut().unwrap().send(Message {
                        src: response.src.expect("response must have a source"),
                        dest: response.dest.expect("response must have a destination"),
                        body: Body {
                            typ: response.typ,
                            msg_id: None, // TODO
                            in_reply_to: message.body.msg_id,
                            extra: response.body.unwrap_or_default(),
                        },
                    });
                }
            });
        }
    }
}

pub static mut PROTOCOL: OnceLock<Protocol> = OnceLock::new();
