use std::{
    collections::HashMap,
    io::{self, Write},
    process::Output,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    task::Waker,
    thread,
    time::Duration,
};

use futures::{
    future::BoxFuture,
    task::{waker_ref, ArcWake},
    Future, FutureExt,
};
use log::{debug, info};
use serde_json::{Map, Value};
use std::collections;

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

/// Pool of threads used for blocking tasks.
struct ThreadPool {
    workers: Vec<Worker>,
    capacity: usize,
}

impl ThreadPool {
    fn new(capacity: usize) -> Self {
        ThreadPool {
            workers: Vec::new(),
            capacity,
        }
    }

    fn spawn_worker_thread(&mut self) {
        if self.workers.len() > self.capacity {
            return;
        }

        // TODO is Box<dyn Fn()> the right type here?
        let (task_sender, task_receiver) = mpsc::channel::<Box<dyn Fn() + Send + 'static>>();
        let (park_signal, park_receiver) = mpsc::channel::<bool>();
        let handle = thread::spawn(move || {
            loop {
                // TODO is this the right timeout value?
                while let Ok(task) = task_receiver.recv_timeout(Duration::from_millis(100)) {
                    task();
                }

                // Seems like no task was received, so we need to park the thread.
                park_signal.send(true).unwrap();
                thread::park();
            }
        });

        self.workers.push(Worker {
            task_sender,
            join_handle: handle,
        });
    }
}

struct Worker {
    task_sender: mpsc::Sender<Box<dyn Fn() + Send + 'static>>,
    join_handle: thread::JoinHandle<()>,
}

/// A basic single threaded async executor. Blocking tasks are executed in a
/// separate thread pool. This is the model Node.js uses.
struct AsyncExecutor {
    queue: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
    blocking_queue: collections::VecDeque<Arc<Task>>,
}

impl AsyncExecutor {
    fn new() -> Self {
        let (sender, queue) = mpsc::channel::<Arc<Task>>();

        let blocking_queue = collections::VecDeque::new();

        Self {
            queue,
            sender,
            blocking_queue,
        }
    }

    fn run(&self) {
        while let Ok(task) = self.queue.recv() {
            debug!("running task");
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let context = &mut std::task::Context::from_waker(&waker);
            let _ = future.as_mut().poll(context);
        }
    }

    fn drop(&mut self) {
        let _ = std::mem::replace(&mut self.sender, mpsc::channel().0);
    }

    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(future),
            task_sender: self.sender.clone(),
        });

        self.sender.send(task).unwrap();
    }

    fn spawn_blocking(&self, future: impl Future<Output = ()> + 'static + Send) {}
}

struct Task {
    future: Mutex<BoxFuture<'static, ()>>,
    task_sender: mpsc::Sender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        debug!("waking task");
        let cloned = arc_self.clone();
        // TODO proper error handling
        arc_self.task_sender.send(cloned).unwrap();
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

    let mut executor = AsyncExecutor::new();
    executor.spawn(async {
        let future1 = TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(1));
        debug!("hello");
        debug!("timer 2 elapsed from main: {:?}", timer().await);
        debug!("timer 1 elapsed from main: {:?}", future1.await);
    });

    debug!("hello from main");

    executor.spawn(async {
        let future1 = TimerThenReturnElapsedFuture::new(std::time::Duration::from_secs(4));
        debug!("timer 3 elapsed from main: {:?}", future1.await);
    });

    // Drop the sender so that the executor will stop when all tasks are done
    executor.drop();

    executor.run();

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
