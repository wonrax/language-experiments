use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, OnceLock, RwLock},
};

use async_runtime::runtime::current;
use futures::{future::join, lock::Mutex, Future, SinkExt, StreamExt};
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
    shared: Mutex<Shared>,
}

struct Shared {
    // global incrementing counter for message ids
    msg_id: u64,

    // a queue of message ids and its response future to wake up
    responses: HashMap<u64, RequestFuture>,
}

impl Protocol {
    pub fn new() -> Self {
        Self {
            shared: Mutex::new(Shared {
                msg_id: 0,
                responses: HashMap::new(),
            }),
        }
    }

    fn write_message(&self, message: Message) {
        let message_str = serde_json::to_string(&message).unwrap() + "\n";
        io::stdout().write_all(message_str.as_bytes()).unwrap();
    }

    // fire and forget
    // TODO spawn a task so that the write is queued instead of blocking
    pub async fn send(&self, mut message: Message) {
        let mut lock = self.shared.lock().await;

        lock.msg_id += 1;
        message.body.msg_id = Some(lock.msg_id);

        drop(lock);

        self.write_message(message);
    }

    // send and wait for response
    // TODO support timeout
    pub async fn call(&self, mut message: Message) -> Response {
        let fut = RequestFuture::new();

        let mut lock = self.shared.lock().await;

        lock.msg_id += 1;
        message.body.msg_id = Some(lock.msg_id);

        lock.responses
            .insert(message.body.msg_id.unwrap(), fut.clone());

        drop(lock);

        self.write_message(message);

        fut.await
    }

    // start accepting messages
    pub async fn listen<F, R>(&self, handler: F)
    where
        F: FnMut(Request) -> R + Send + Clone + 'static,
        R: Future<Output = Response> + Send,
    {
        let mut stdin_listener = StdinListener::new();
        let mut this_node_id: Option<String> = None;

        loop {
            let message: Message =
                serde_json::from_str(&stdin_listener.next().await.unwrap()).unwrap();

            if this_node_id.is_none() && message.body.typ == "init" {
                this_node_id = Some(message.body.extra["node_id"].as_str().unwrap().to_string());
            }

            // TODO find optimization opportunity here (e.g. spawn task)
            if let Some(ref in_reply_to) = message.body.in_reply_to {
                if let Some(ref this_node_id) = this_node_id {
                    let mut lock = self.shared.lock().await;

                    if this_node_id == &message.dest && lock.responses.contains_key(in_reply_to) {
                        lock.responses
                            .remove(in_reply_to)
                            .unwrap()
                            .set_response(Response {
                                typ: message.body.typ,
                                src: Some(message.src),
                                dest: Some(message.dest),
                                body: Some(message.body.extra),
                            });

                        continue;
                    }
                }
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

                PROTOCOL
                    .get()
                    .expect(
                        "Protocol should've been initialized by now. \
                            Either the protocol is not initialized or \
                            you're trying to use the protocol from a \
                            different thread.",
                    )
                    // .unwrap()
                    .send(Message {
                        src: response.src.expect("response must have a source"),
                        dest: response.dest.expect("response must have a destination"),
                        body: Body {
                            typ: response.typ,
                            msg_id: None, // TODO
                            in_reply_to: message.body.msg_id,
                            extra: response.body.unwrap_or_default(),
                        },
                    })
                    .await;
            });
        }
    }
}

pub static PROTOCOL: OnceLock<Protocol> = OnceLock::new();
