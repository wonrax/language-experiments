use std::{
    cell::OnceCell,
    collections::HashMap,
    io::{self, stdin, Write},
    pin::Pin,
    rc::Rc,
    sync::{mpsc, Arc, Mutex, OnceLock},
    task::{Poll, Waker},
    thread,
    time::Duration,
};

use async_runtime::runtime::{current, new_runtime};
use futures::{Future, Stream, StreamExt};
use log::{debug, info};
use serde_json::{Map, Value};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Message {
    src: String,
    dest: String,
    body: Body,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Body {
    #[serde(rename = "type")]
    typ: String,
    msg_id: Option<u64>,
    in_reply_to: Option<u64>,

    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Clone)]
struct Context {
    node_id: Rc<String>,
    node_ids: Rc<Vec<String>>,
}

#[derive(Debug)]
struct Request {
    typ: String,
    src: Option<String>,
    dest: Option<String>,
    body: Option<Map<String, Value>>,
}

type Response = Request;

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

// Listens to stdin and read lines in a loop.
struct StdinListener {
    waker: Arc<Mutex<Option<Waker>>>,
    recv: crossbeam_channel::Receiver<String>,
    send: crossbeam_channel::Sender<String>,
}

// unsafe impl Send for AsyncStdinListener<'_> {}

impl StdinListener {
    fn new() -> Self {
        let (send, recv) = crossbeam_channel::unbounded::<String>();

        Self {
            waker: Arc::new(Mutex::new(None)),
            recv,
            send,
        }
    }
}

impl Stream for StdinListener {
    type Item = String;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        debug!("polling async stdin");

        if self.recv.is_empty() {
            let mut r = self.waker.lock().unwrap();
            *r = Some(cx.waker().clone());

            let waker = cx.waker().clone();
            let send = self.send.clone();

            // start the operation in a separate thread
            let runtime = current();
            runtime.spawn_blocking(move || {
                let stdin = stdin();
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).unwrap();

                debug!("sending line: {}", buffer);
                send.send(buffer).unwrap();
                waker.wake();
            });

            debug!("async stdin returning pending");
            return Poll::Pending;
        }

        match self.recv.recv() {
            Ok(value) => Poll::Ready(Some(value)),
            Err(_) => Poll::Pending,
        }
    }
}

struct TimerThenReturnElapsedFuture {
    state: Arc<Mutex<(bool, Option<Waker>, Option<std::time::Duration>)>>,
    duration: std::time::Duration,
}

impl TimerThenReturnElapsedFuture {
    fn new(duration: std::time::Duration) -> Self {
        let state = Arc::new(Mutex::new((false, Option::<Waker>::None, None)));

        let state_clone = state.clone();
        thread::spawn(move || {
            let timer = std::time::Instant::now();
            std::thread::sleep(duration);
            let mut state = state_clone.lock().unwrap();
            state.0 = true;
            state.2 = Some(timer.elapsed());
            if let Some(waker) = state.1.take() {
                waker.wake();
            }
        });

        Self { state, duration }
    }
}

impl Future for TimerThenReturnElapsedFuture {
    type Output = std::time::Duration;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        debug!("polling timer with duration {:?}", self.duration);
        let mut state = self.state.lock().unwrap();
        if state.0 {
            return std::task::Poll::Ready(state.2.unwrap());
        }

        state.1 = Some(cx.waker().to_owned());
        std::task::Poll::Pending
    }
}

async fn timer() -> std::time::Duration {
    TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(3)).await
}

// RPC protocol
#[derive(Debug)]
struct RPC {
    // A lock to ensure only one task is writing to stdout at a time
    write_lock: Arc<Mutex<()>>,

    // global incrementing counter for message ids
    msg_id: u64,
}

impl RPC {
    fn new() -> Self {
        Self {
            write_lock: Arc::new(Mutex::new(())),
            msg_id: 0,
        }
    }

    fn send(&mut self, mut message: Message) {
        self.msg_id += 1;
        message.body.msg_id = Some(self.msg_id);
        let message_str = serde_json::to_string(&message).unwrap() + "\n";
        let _lock = self.write_lock.lock().unwrap();
        io::stdout().write_all(message_str.as_bytes()).unwrap();
    }
}

static mut RPC_INSTANCE: OnceLock<RPC> = OnceLock::new();

fn main() {
    pretty_env_logger::init_timed();

    unsafe {
        RPC_INSTANCE.set(RPC::new()).unwrap();
    }

    let mut runtime = new_runtime(4, 36);

    runtime.block_on(async {
        let mut stdin_listener = StdinListener::new();

        let init = stdin_listener.next().await.unwrap();
        let message: Message = serde_json::from_str(&init).unwrap();
        if message.body.typ != "init" {
            panic!("first message must be init");
        }
        loop {
            let message: Message =
                serde_json::from_str(&stdin_listener.next().await.unwrap()).unwrap();

            // TODO check if the dest is this node

            current().spawn(async move {
                let response = handler(Request {
                    typ: message.body.typ,
                    src: Some(message.src.clone()),
                    dest: Some(message.dest.clone()),
                    body: Some(message.body.extra),
                })
                .await;

                unsafe {
                    RPC_INSTANCE.get_mut().unwrap().send(Message {
                        src: message.dest,
                        dest: message.src,
                        body: Body {
                            typ: response.typ,
                            msg_id: None, // TODO
                            in_reply_to: message.body.msg_id,
                            extra: response.body.unwrap(),
                        },
                    });
                }
            });
        }
    });
}

// handler receives message and returns a message as a response
async fn handler(r: Request) -> Response {
    debug!("got message: {:?}", r);
    Response {
        typ: "echo_ok".into(),
        src: None,
        dest: None,
        body: r.body,
    }
}
