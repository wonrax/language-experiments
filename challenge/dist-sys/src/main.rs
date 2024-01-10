use std::{
    collections::HashMap,
    io::{self, stdin, Write},
    pin::Pin,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    task::{Poll, Waker},
    thread,
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

#[derive(Clone)]
struct Context {
    send: mpsc::Sender<Request>,
    node_id: Rc<String>,
    node_ids: Rc<Vec<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Body {
    #[serde(rename = "type")]
    typ: String,
    msg_id: Option<i64>,
    in_reply_to: Option<i64>,

    #[serde(flatten)]
    extra: Map<String, Value>,
}

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

struct Request {
    msg: Message,

    // Use this to send a response back to the client
    send: mpsc::Sender<Message>,
}

struct AsyncStdinListener {
    waker: Arc<Mutex<Option<Waker>>>,
    recv: crossbeam_channel::Receiver<String>,
    send: crossbeam_channel::Sender<String>,
}

// unsafe impl Send for AsyncStdinListener<'_> {}

impl AsyncStdinListener {
    fn new() -> Self {
        let (send, recv) = crossbeam_channel::unbounded::<String>();

        Self {
            waker: Arc::new(Mutex::new(None)),
            recv,
            send,
        }

        // let waker = this.waker.clone();

        // TODO this thread won't stop running even if the future is dropped.
        // thread_pool.spawn_blocking(move || loop {
        //     let stdin = stdin();
        //     let mut buffer = String::new();
        //     stdin.read_line(&mut buffer).unwrap();

        //     debug!("sending line: {}", buffer);
        //     match send.try_send(buffer) {
        //         Err(e) => match e {
        //             crossbeam_channel::TrySendError::Full(_) => {
        //                 debug!("channel full, dropping line");
        //             }
        //             crossbeam_channel::TrySendError::Disconnected(_) => {
        //                 debug!("channel disconnected, exiting thread");
        //                 break; // break and exit the thread
        //             }
        //         },
        //         _ => {}
        //     }
        //     if let Some(waker) = waker.lock().unwrap().take() {
        //         waker.wake();
        //     }
        // });
    }
}

impl Stream for AsyncStdinListener {
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

fn main() {
    pretty_env_logger::init_timed();

    let mut runtime = new_runtime();

    let mut stdin_listener = AsyncStdinListener::new();
    runtime.spawn(async move {
        for _ in 0..2 {
            debug!("got line: {}", stdin_listener.next().await.unwrap());
        }
    });

    runtime.spawn(async {
        let future1 = TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(1));
        debug!("hello");
        debug!("timer 2 elapsed from main: {:?}", timer().await);
        debug!("timer 1 elapsed from main: {:?}", future1.await);
    });

    debug!("hello from main");

    runtime.spawn(async {
        let future1 = TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(4));
        debug!("timer 3 elapsed from main: {:?}", future1.await);
    });

    runtime.block_on(async {
        let future1 = TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(10));
        future1.await;
    });

    // Drop the sender so that the executor will stop when all tasks are done
    // runtime.drop();

    debug!("done");

    let mut buffer = String::new();

    io::stdin().read_line(&mut buffer).unwrap();
    let message: Message = serde_json::from_str(&buffer).unwrap();
    if message.body.typ != "init" {
        panic!("first message must be init");
    }

    // use this as a RPC client, which is different than the main loop below
    // which acts like a listen server
    let (send_request, recv_request) = mpsc::channel::<Request>();

    let mut request_senders: Arc<Mutex<HashMap<i64, mpsc::Sender<Message>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let request_senders_clone = request_senders.clone();

    thread::spawn(move || {
        for request in recv_request {
            let request_str = serde_json::to_string(&request.msg).unwrap() + "\n";
            io::stdout().write_all(request_str.as_bytes()).unwrap();

            let mut request_senders = request_senders_clone.lock().unwrap();
            request_senders.insert(request.msg.body.msg_id.unwrap(), request.send);
        }
    });

    let context = Context {
        send: send_request,
        node_id: Rc::new(
            message
                .body
                .extra
                .get("node_id")
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
        ),
        node_ids: Rc::new(
            message
                .body
                .extra
                .get("node_ids")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
        ),
    };

    buffer.clear();

    let request_senders_clone = request_senders.clone();
    let response_buffer = String::new();

    thread::spawn(|| {});

    while let Ok(length) = io::stdin().read_line(&mut buffer) {
        let message: Message = serde_json::from_str(&buffer).unwrap();

        let response = match message.body.typ.as_str() {
            "echo" => handle_echo(&message, context.clone()).map(|r| Some(r)),
            _ => Ok(None),
        };

        let res = match response {
            Ok(r) => r,
            Err(e) => Some(Message::from(e)),
        };

        if let Some(res) = res {
            let response_str = serde_json::to_string(&res).unwrap() + "\n";
            io::stdout().write_all(response_str.as_bytes()).unwrap();
        }

        buffer.clear();
    }
}

fn handle_echo(msg: &Message, context: Context) -> Result<Message, anyhow::Error> {
    let echo = msg
        .body
        .extra
        .get("echo")
        .ok_or(anyhow::anyhow!(
            "message is type echo but no echo field present"
        ))?
        .clone();

    let response = Message {
        src: "n1".into(),
        dest: "c1".into(),
        body: Body {
            typ: "echo_ok".into(),
            msg_id: Some(1),
            in_reply_to: msg.body.msg_id,
            extra: Map::from_iter([("echo".to_string(), echo)]),
        },
    };

    let (message_send, message_recv) = mpsc::channel::<Message>();

    context.send.send(Request {
        msg: Message {
            src: "node".into(),
            dest: "me".into(),
            body: Body {
                typ: "echo".into(),
                msg_id: Some(69),
                in_reply_to: None,
                extra: Map::new(),
            },
        },
        send: message_send,
    });

    for message in message_recv {
        debug!("got message: {:?}", message);
    }

    Ok(response)
}
