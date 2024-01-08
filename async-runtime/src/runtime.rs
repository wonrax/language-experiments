use futures::{
    task::{waker_ref, ArcWake},
    Future,
};
use log::{debug, info};
use std::{
    cell::RefCell,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::threadpool::{JoinHandle, ThreadPool};

thread_local! {
    static EXECUTOR: RefCell<Option<AsyncRuntime>> = RefCell::new(None);
}

pub fn get_runtime() -> AsyncRuntime {
    EXECUTOR.with(|executor| {
        executor
            .borrow()
            .clone()
            .expect("The async runtime is None, maybe you forgot to make one")
    })
}

pub fn new_runtime() -> AsyncRuntime {
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
pub struct AsyncRuntime {
    inner: Arc<Inner>,
    thread_pool: Arc<ThreadPool>,
}

impl AsyncRuntime {
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

    fn queue_task(&self, task: Arc<Box<dyn TaskMarker + Sync + Send>>) {
        self.inner.queue_task(task);
    }
}

struct Inner {
    queue: crossbeam_channel::Receiver<Arc<Box<dyn TaskMarker + Sync + Send>>>,
    sender: crossbeam_channel::Sender<Arc<Box<dyn TaskMarker + Sync + Send>>>,
}

impl Inner {
    fn new() -> Self {
        let (sender, queue) =
            crossbeam_channel::unbounded::<Arc<Box<dyn TaskMarker + Sync + Send>>>();
        Self { queue, sender }
    }

    fn queue_task(&self, task: Arc<Box<dyn TaskMarker + Sync + Send>>) {
        self.sender.send(task).unwrap();
    }

    fn run(&self) {
        while let Ok(task) = self.queue.recv() {
            debug!("running task");
            task.poll();
        }
        debug!("async executor stopped")
    }

    fn stop(&mut self) {
        let _ = std::mem::replace(&mut self.sender, crossbeam_channel::unbounded().0);
    }

    /// Future is not needed to be Send since we're doing single threaded but
    /// the ArcWake trait requires it for more general use cases.
    fn spawn<R>(&self, future: impl Future<Output = R> + 'static + Send) -> JoinHandle<R>
    where
        R: std::any::Any + Send + 'static,
    {
        let future = Box::pin(future);

        let (result_send, result_recv) = crossbeam_channel::bounded(1);

        let task: Arc<Box<dyn TaskMarker + Send + Sync>> = Arc::new(Box::new(Task {
            future: Mutex::new(future),
            task_sender: self.sender.clone(),
            result_sender: Some(result_send),
        }));

        self.sender.send(task).unwrap();

        JoinHandle::new(result_recv)
    }
}

struct Task<'a, R>
where
    R: std::any::Any + Send + 'static,
{
    future: Mutex<Pin<Box<dyn Future<Output = R> + Send + 'a>>>,
    task_sender: crossbeam_channel::Sender<Arc<Box<dyn TaskMarker + Sync + Send>>>,
    result_sender: Option<crossbeam_channel::Sender<Box<dyn std::any::Any + Send + 'static>>>,
}

trait TaskMarker: Send {
    fn poll(&self);
}

impl<R> TaskMarker for Task<'static, R>
where
    R: std::any::Any + Send + 'static,
{
    fn poll(&self) {
        let boxed: Box<dyn TaskMarker + Sync + Send> = Box::new(self);
        let mut future = self.future.lock().unwrap();
        let waker = waker_ref(&Arc::new(boxed));
        let context = &mut std::task::Context::from_waker(&waker);
        let _ = future.as_mut().poll(context);
    }
}

impl<T> ArcWake for Task<'static, T>
where
    T: std::any::Any + Send + 'static,
{
    fn wake_by_ref(arc_self: &Arc<Self>) {
        debug!("waking task");
        let boxed: Box<dyn TaskMarker + Sync + Send> = Box::new(arc_self);
        get_runtime().queue_task(arc_self.clone())
    }
}
// impl ArcWake for Box<dyn TaskMarker + Sync + Send> {
//     fn wake_by_ref(arc_self: &Arc<Self>) {
//         debug!("waking task");
//         let cloned = arc_self.clone();
//         // TODO proper error handling
//         arc_self.wake();
//     }
// }
