use std::{
    sync::{Arc, Mutex},
    task::Waker,
};

use futures::Future;

use crate::proto::{Body, Message, Request, Response, RPC, RPC_INSTANCE};

#[derive(Debug)]
struct Inner {
    response: Option<Response>,
    waker: Option<Waker>,
}

#[derive(Debug, Clone)]
pub struct RequestFuture(Arc<Mutex<Inner>>);

impl RequestFuture {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            response: None,
            waker: None,
        })))
    }

    pub fn set_response(&mut self, response: Response) {
        let mut inner = self.0.lock().unwrap();
        inner.response = Some(response);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

impl Future for RequestFuture {
    type Output = Response;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut inner = self.0.lock().unwrap();
        if let Some(response) = inner.response.take() {
            return std::task::Poll::Ready(response);
        }

        inner.waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}

// TODO return error instead
pub async fn request(r: Request) -> Response {
    let message = Message {
        src: "TODO".into(),
        dest: r.dest.unwrap(),
        body: Body {
            typ: r.typ,
            msg_id: None,
            in_reply_to: None,
            extra: r.body.unwrap_or_default(),
        },
    };

    let future = RequestFuture::new();

    unsafe {
        // TODO not efficient
        let _ = RPC_INSTANCE.set(RPC::new());

        RPC_INSTANCE
            .get_mut()
            .expect("RPC instance not initialized")
            .call(message, future)
            .await
            .unwrap()
    }
}
