use log::debug;
use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::runtime::{current, set_current};

struct BlockingTask {
    task: Box<dyn FnOnce() -> Box<dyn std::any::Any + Send + 'static> + Send>,
    result: Option<crossbeam_channel::Sender<Box<dyn std::any::Any + Send + 'static>>>,
}

pub struct JoinHandle<R>(
    crossbeam_channel::Receiver<Box<dyn std::any::Any + Send + 'static>>,
    PhantomData<R>,
)
where
    R: std::any::Any + Send + 'static;

impl<R> JoinHandle<R>
where
    R: std::any::Any + Send + 'static,
{
    pub fn new(
        result_recv: crossbeam_channel::Receiver<Box<dyn std::any::Any + Send + 'static>>,
    ) -> Self {
        JoinHandle(result_recv, PhantomData)
    }

    pub fn join(self) -> R {
        *self.0.recv().unwrap().downcast().unwrap()
    }
}

/// Pool of threads used for blocking tasks.
pub struct ThreadPool {
    capacity: usize,
    task_recv: crossbeam_channel::Receiver<BlockingTask>,
    task_send: crossbeam_channel::Sender<BlockingTask>,
    num_threads: Arc<AtomicUsize>,
}

impl ThreadPool {
    pub fn new(capacity: usize) -> Self {
        let (task_send, task_recv) = crossbeam_channel::unbounded();
        ThreadPool {
            capacity,
            task_recv,
            task_send,
            num_threads: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn spawn_blocking<F, R>(&self, task: F) -> JoinHandle<R>
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

        // get the current runtime handle and pass it to the thread
        let handle = current();

        thread::Builder::new()
            .name("blocking_thread".into())
            .spawn(move || {
                debug!("setting runtime handle");
                set_current(handle);

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
                                // closed before the result is sent
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
