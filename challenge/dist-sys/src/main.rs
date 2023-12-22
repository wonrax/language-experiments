use std::{
    borrow::BorrowMut,
    cell::Cell,
    collections::HashMap,
    io::{self, stdin, Write},
    marker::PhantomData,
    pin::Pin,
    process::Output,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    task::{Poll, Waker},
    thread,
    time::Duration,
};

use futures::{
    future::BoxFuture,
    sink::Unfold,
    stream::{unfold, ForEachConcurrent},
    task::{waker_ref, ArcWake},
    Future, FutureExt, Stream, StreamExt,
};
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

fn handle_protocol_stdio(thread_pool: &ThreadPool) -> Pin<Box<dyn Stream<Item = String> + Send>> {
    let (send, recv) = crossbeam_channel::unbounded::<String>();

    let stdin = stdin();
    thread_pool.spawn_blocking(move || loop {
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();

        send.send(buffer.clone()).unwrap();
        debug!("sent line: {}", buffer);
        buffer.clear();
    });

    Box::pin(unfold(recv, |recv| async {
        let result = recv.recv().unwrap();

        Some((result, recv))
    }))
    // .for_each_concurrent(None, |line| async move {
    //     debug!("got line: {}", line);
    // })
}

struct AsyncStdinListener<'a> {
    waker: Arc<Mutex<Option<Waker>>>,
    recv: crossbeam_channel::Receiver<String>,
    send: crossbeam_channel::Sender<String>,
    executor: AsyncExecutor<'a>,
}

unsafe impl Send for AsyncStdinListener<'_> {}

impl<'a> AsyncStdinListener<'a> {
    fn new(executor: AsyncExecutor<'a>) -> Self {
        let (send, recv) = crossbeam_channel::unbounded::<String>();

        Self {
            waker: Arc::new(Mutex::new(None)),
            recv,
            send,
            executor,
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

impl Stream for AsyncStdinListener<'_> {
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
            self.executor.spawn_blocking(move || {
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

struct BlockingTask {
    task: Box<dyn FnOnce() -> Box<dyn std::any::Any + Send + 'static> + Send>,
    result: Option<crossbeam_channel::Sender<Box<dyn std::any::Any + Send + 'static>>>,
}

struct JoinHandle<R>(
    crossbeam_channel::Receiver<Box<dyn std::any::Any + Send + 'static>>,
    PhantomData<R>,
)
where
    R: std::any::Any + Send + 'static;

impl<R> JoinHandle<R>
where
    R: std::any::Any + Send + 'static,
{
    fn join(self) -> R {
        *self.0.recv().unwrap().downcast().unwrap()
    }
}

/// Pool of threads used for blocking tasks.
#[derive(Clone)]
struct ThreadPool {
    capacity: usize,
    task_send: crossbeam_channel::Sender<BlockingTask>,
    task_recv: crossbeam_channel::Receiver<BlockingTask>,
    num_threads: Arc<AtomicUsize>,
}

// unsafe impl Sync for ThreadPool {}

impl ThreadPool {
    fn new(capacity: usize) -> Self {
        let (task_send, task_recv) = crossbeam_channel::unbounded();
        ThreadPool {
            capacity,
            task_send,
            task_recv,
            num_threads: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn spawn_blocking<F, R>(&self, task: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: std::any::Any + Send + 'static,
    {
        // TODO for correctness, mutex should be used here
        let (result_send, result_recv) = crossbeam_channel::bounded(1);

        self.task_send
            .send(BlockingTask {
                task: Box::new(|| Box::new(task())),
                result: Some(result_send),
            })
            .unwrap();

        if self.num_threads.load(Ordering::Relaxed) < self.capacity {
            self.spawn_thread();
        }

        // *result_recv.recv().unwrap().downcast::<R>().unwrap()
        JoinHandle(result_recv, PhantomData)
    }

    fn spawn_thread(&self) {
        debug!("spawning new thread");
        let task_recv = self.task_recv.clone();

        // TODO is Box<dyn Fn()> the right type here?
        self.num_threads.fetch_add(1, Ordering::Relaxed);
        let num_threads = self.num_threads.clone();
        thread::Builder::new()
            .name("blocking_thread".into())
            .spawn(move || {
                debug!("blocking thread started");
                loop {
                    // TODO is this the right timeout value?
                    match task_recv.recv_timeout(Duration::from_millis(100)) {
                        Ok(task) => {
                            debug!("blocking thread pool received new task");
                            let result = (task.task)();
                            if let Some(result_sender) = task.result {
                                // ignore the error because there are cases
                                // where the caller doesn't need the JoinHandle
                                // thus it's dropped and the result channel is
                                // closed
                                let _ = result_sender.send(result);
                            }
                        }
                        Err(_) => break, // break and exit the thread
                    }
                }

                debug!("blocking thread exiting");
                num_threads.fetch_sub(1, Ordering::Relaxed);
            })
            .unwrap();
    }
}

/// A basic single threaded async executor. Blocking tasks are executed in a
/// separate thread pool. This is the model Node.js uses. In the future, this
/// will be evolved to be a multi-threaded executor, but for now we want to test
/// for the correctness of the async implementation first.
#[derive(Clone)]
struct AsyncExecutor<'a> {
    queue: crossbeam_channel::Receiver<Arc<Task<'a>>>,
    sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
    thread_pool: ThreadPool,
}

impl<'a> AsyncExecutor<'a> {
    fn new() -> Self {
        let (sender, queue) = crossbeam_channel::unbounded::<Arc<Task>>();
        // TODO make this configurable
        let thread_pool = ThreadPool::new(4);

        Self {
            queue,
            sender,
            thread_pool,
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
        let _ = std::mem::replace(&mut self.sender, crossbeam_channel::unbounded().0);
    }

    /// Future is not needed to be Send since we're doing single threaded but
    /// the ArcWake trait requires it for more general use cases.
    fn spawn(&self, future: impl Future<Output = ()> + 'a + Send) {
        let future = Box::pin(future);
        let task = Arc::new(Task {
            future: Mutex::new(future),
            task_sender: self.sender.clone(),
        });

        self.sender.send(task).unwrap();
    }

    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: std::any::Any + Send + 'static,
    {
        self.thread_pool.spawn_blocking(f)
    }

    fn test() -> impl Send {
        ()
    }
}

struct Task<'a> {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send + 'a>>>,
    task_sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
}

impl ArcWake for Task<'_> {
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

    let thread_pool = ThreadPool::new(4);

    debug!("spawning task");
    let join = thread_pool.spawn_blocking(|| -> Result<i32, ()> {
        debug!("blocking task");
        std::thread::sleep(Duration::from_secs(1));
        debug!("blocking task done");

        Ok(1)
    });

    let result = join.join();

    debug!("blocking task result: {:?}", result);

    let mut executor = AsyncExecutor::new();

    let mut stdin_listener = AsyncStdinListener::new(executor.clone());
    executor.spawn(async move {
        for _ in 0..2 {
            debug!("got line: {}", stdin_listener.next().await.unwrap());
        }
    });

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
    // executor.drop();

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
