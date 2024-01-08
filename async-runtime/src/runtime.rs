use futures::{
    task::{waker_ref, ArcWake},
    Future,
};
use log::{debug, info};
use std::{
    any::Any,
    borrow::BorrowMut,
    cell::{Ref, RefCell},
    os::unix::thread,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::threadpool::{JoinHandle, ThreadPool};

thread_local! {
    static EXECUTOR: RefCell<Option<AsyncRuntime<'static>>> = RefCell::new(None);
}

pub fn get_runtime() -> AsyncRuntime<'static> {
    EXECUTOR.with(|executor| {
        executor
            .borrow()
            .clone()
            .expect("The async runtime is None, maybe you forgot to make one")
    })
}

pub fn new_runtime() -> AsyncRuntime<'static> {
    let runtime = AsyncRuntime::new();
    EXECUTOR.with(|executor| {
        *executor.borrow_mut() = Some(runtime.clone());
    });
    runtime
}

/// A basic single threaded async executor. Blocking tasks are executed in a
/// separate thread pool. This is the model Node.js uses. In the future, this
/// will be evolved to be a multi-threaded executor, but for now we want to test
/// for the correctness of the async implementation first.
#[derive(Clone)]
pub struct AsyncRuntime<'a> {
    inner: Arc<Inner<'a>>,
    thread_pool: Arc<ThreadPool>,
}

impl AsyncRuntime<'static> {
    pub fn new() -> Self {
        let inner = Arc::new(Inner::new());
        let thread_pool = Arc::new(ThreadPool::new(4));
        let runtime = Self { inner, thread_pool };

        runtime
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        self.inner.spawn(future);
    }

    pub fn spawn_blocking<F, R>(&self, task: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: std::any::Any + Send + 'static,
    {
        self.thread_pool.spawn_blocking(task)
    }

    pub fn run(&self) {
        self.inner.run();
    }

    pub fn queue_task(&self, task: Arc<Task<'static>>) {
        self.inner.queue_task(task);
    }
}

struct Inner<'a> {
    queue: crossbeam_channel::Receiver<Arc<Task<'a>>>,
    sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
}

impl Inner<'static> {
    fn new() -> Self {
        let (sender, queue) = crossbeam_channel::unbounded::<Arc<Task>>();
        Self { queue, sender }
    }

    fn run(&self) {
        while let Ok(task) = self.queue.recv() {
            debug!("running task");
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let context = &mut std::task::Context::from_waker(&waker);

            match future.as_mut().poll(context) {
                std::task::Poll::Pending => {
                    debug!("task not ready");
                }
                std::task::Poll::Ready(result) => {
                    debug!("task finished");
                    if let Some(result_sender) = &task.result_sender {
                        result_sender.send(result).unwrap();
                    }
                }
            }
        }
        debug!("async executor stopped")
    }

    fn stop(&mut self) {
        let _ = std::mem::replace(&mut self.sender, crossbeam_channel::unbounded().0);
    }

    /// Future is not needed to be Send since we're doing single threaded but
    /// the ArcWake trait requires it for more general use cases.
    fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> JoinHandle<R>
    where
        R: Send + 'static,
    {
        let future = Box::pin(async {
            let boxed: Box<dyn Any + Send + 'static> = Box::new(future.await);
            boxed
        });

        let (result_send, result_recv) = crossbeam_channel::bounded(1);

        let task = Arc::new(Task {
            future: Mutex::new(future),
            result_sender: Some(result_send),
        });

        self.sender.send(task).unwrap();

        JoinHandle::new(result_recv)
    }

    fn queue_task(&self, task: Arc<Task<'static>>) {
        self.sender.send(task).unwrap();
    }
}

struct Task<'a> {
    future: Mutex<Pin<Box<dyn Future<Output = Box<dyn Any + Send + 'static>> + Send + 'a>>>,
    result_sender: Option<crossbeam_channel::Sender<Box<dyn Any + Send + 'static>>>,
}

impl ArcWake for Task<'static> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        debug!("waking task");
        let cloned = arc_self.to_owned();
        // TODO proper error handling
        get_runtime().queue_task(cloned);
    }
}
