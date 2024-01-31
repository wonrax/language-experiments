use std::{
    sync::{Arc, Mutex},
    task::Waker,
    thread,
};

use futures::Future;
use log::debug;

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
