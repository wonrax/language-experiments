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
    static HANDLE: RefCell<Option<Handle>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct Handle {
    task_sender: crossbeam_channel::Sender<Arc<Task<'static>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Handle {
    fn new(
        task_sender: crossbeam_channel::Sender<Arc<Task<'static>>>,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        Self {
            task_sender,
            thread_pool,
        }
    }

    /// Future is not needed to be Send since we're doing single threaded but
    /// the ArcWake trait requires it for more general use cases.
    pub fn spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> JoinHandle<R>
    where
        R: Send + 'static,
    {
        let future = Box::pin(async {
            let boxed: Box<TaskResult> = Box::new(future.await);
            boxed
        });

        let (result_send, result_recv) = crossbeam_channel::bounded(1);

        let task = Arc::new(Task {
            future: Mutex::new(future),
            task_sender: self.task_sender.clone(),
            result_sender: Some(result_send),
        });

        self.task_sender.send(task).unwrap();

        JoinHandle::new(result_recv)
    }

    pub fn spawn_blocking<F, R>(&self, task: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: std::any::Any + Send + 'static,
    {
        self.thread_pool.spawn_blocking(task)
    }
}

pub fn current() -> Handle {
    HANDLE.with(|handle| {
        handle
            .borrow()
            .clone()
            .expect("The async runtime is None, maybe you forgot to make one")
    })
}

pub fn set_current(handle: Handle) {
    HANDLE.with(|h| {
        *h.borrow_mut() = Some(handle);
    });
}

pub fn new_runtime() -> Handle {
    let executor = Executor::new();
    let thread_pool = Arc::new(ThreadPool::new(4));

    let handle = Handle::new(executor.sender.clone(), thread_pool.clone());

    set_current(handle.clone());

    thread_pool.spawn_blocking(move || executor.run());

    handle
}

/// A basic single threaded async executor. Blocking tasks are executed in a
/// separate thread pool. This is the model Node.js uses. In the future, this
/// will be evolved to be a multi-threaded executor, but for now we want to test
/// for the correctness of the async implementation first.
struct Executor<'a> {
    queue: crossbeam_channel::Receiver<Arc<Task<'a>>>,
    sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
}

// TODO implement lifetime correctly
impl Executor<'static> {
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
                        // ignore the error because there are cases
                        // where the caller doesn't need the JoinHandle
                        // thus it's dropped and the result channel is
                        // closed
                        let _ = result_sender.send(result);
                    }
                }
            }
        }
        debug!("async executor stopped")
    }

    fn stop(&mut self) {
        let _ = std::mem::replace(&mut self.sender, crossbeam_channel::unbounded().0);
    }
}

type TaskResult = dyn Any + Send + 'static;

struct Task<'a> {
    future: Mutex<Pin<Box<dyn Future<Output = Box<TaskResult>> + Send + 'a>>>,
    task_sender: crossbeam_channel::Sender<Arc<Task<'a>>>,
    result_sender: Option<crossbeam_channel::Sender<Box<TaskResult>>>,
}

impl ArcWake for Task<'static> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        debug!("waking task");
        let cloned = arc_self.to_owned();
        // TODO proper error handling
        arc_self.task_sender.send(cloned).unwrap();
    }
}
