mod proto;
mod server;
mod stdin;
mod timer;

use std::rc::Rc;

use async_runtime::runtime::new_runtime;
use log::debug;
use proto::{Request, Response};
use server::listen;

#[derive(Clone)]
struct Context {
    node_id: Rc<String>,
    node_ids: Rc<Vec<String>>,
}

fn main() {
    pretty_env_logger::init_timed();

    let runtime = new_runtime(4, 36);

    runtime.block_on(listen(handler));
}

// handler receives message and returns a message as a response
async fn handler(r: Request) -> Response {
    debug!("got message: {:?}", r);
    Response {
        typ: "echo_ok".into(),
        src: None,
        dest: None,
        body: r.body,
    }
}
