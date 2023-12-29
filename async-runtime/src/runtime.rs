use futures::{
    task::{waker_ref, ArcWake},
    Future,
};
use log::{debug, info};
use std::{
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
}

struct Inner<'a> {
    queue: crossbeam_channel::Receiver<Arc<Task<'a>>>,
    sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
}

impl<'a> Inner<'a> {
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
            let _ = future.as_mut().poll(context);
        }
        debug!("async executor stopped")
    }

    fn stop(&mut self) {
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
