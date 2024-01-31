use std::{
    io::stdin,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use async_runtime::runtime::current;
use futures::Stream;
use log::debug;

// Listens to stdin and read lines in a loop.
pub struct StdinListener {
    waker: Arc<Mutex<Option<Waker>>>,
    recv: crossbeam_channel::Receiver<String>,
    send: crossbeam_channel::Sender<String>,
}

// unsafe impl Send for AsyncStdinListener<'_> {}

impl StdinListener {
    pub fn new() -> Self {
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
