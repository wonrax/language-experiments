use async_runtime::runtime::current;
use futures::{Future, StreamExt as _};

use crate::{
    proto::{Body, Message, RPC, RPC_INSTANCE},
    stdin::StdinListener,
    Request, Response,
};

pub async fn listen<F, R>(handler: F)
where
    F: Fn(Request) -> R + Send + Copy + 'static,
    R: Future<Output = Response> + Send,
{
    unsafe {
        RPC_INSTANCE.set(RPC::new()).unwrap();
    }

    let mut stdin_listener = StdinListener::new();

    let init = stdin_listener.next().await.unwrap();
    let message: Message = serde_json::from_str(&init).unwrap();
    if message.body.typ != "init" {
        panic!("first message must be init");
    }
    loop {
        let message: Message = serde_json::from_str(&stdin_listener.next().await.unwrap()).unwrap();

        // TODO check if the dest is this node

        current().spawn(async move {
            let response = handler(Request {
                typ: message.body.typ,
                src: Some(message.src.clone()),
                dest: Some(message.dest.clone()),
                body: Some(message.body.extra),
            })
            .await;

            unsafe {
                RPC_INSTANCE.get_mut().unwrap().send(Message {
                    src: message.dest,
                    dest: message.src,
                    body: Body {
                        typ: response.typ,
                        msg_id: None, // TODO
                        in_reply_to: message.body.msg_id,
                        extra: response.body.unwrap(),
                    },
                });
            }
        });
    }
}
