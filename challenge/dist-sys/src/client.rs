use crate::proto::{Body, Message, Request, Response, RPC, RPC_INSTANCE};

// TODO return error instead
pub fn request(r: Request) -> Response {
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

    unsafe {
        // TODO not efficient
        let _ = RPC_INSTANCE.set(RPC::new());

        RPC_INSTANCE
            .get_mut()
            .expect("RPC instance not initialized")
            .call(message)
            .unwrap()
    }
}
