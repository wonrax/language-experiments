mod client;
mod proto;
mod server;
mod stdin;
mod timer;

use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use async_runtime::runtime::new_runtime;
use client::request;
use log::debug;
use proto::{Request, Response};
use server::listen;

#[derive(Clone)]
struct App {
    context: Arc<RwLock<Option<Context>>>,
}

#[derive(Debug)]
struct Context {
    node_id: String,
    node_ids: Vec<String>,
}

impl App {
    // handler receives message and returns a message as a response
    async fn handler(self, r: Request) -> Response {
        if r.typ == "init" {
            let mut context = self.context.write().unwrap();
            if context.is_some() {
                panic!("context already initialized");
            }
            let node_id = r.src.unwrap();
            let node_ids = r.body.unwrap()["node_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect();
            *context = Some(Context { node_id, node_ids });
            debug!("initialized context: {:?}", context);
            return Response {
                typ: "init_ok".into(),
                src: None,
                dest: None,
                body: None,
            };
        }

        Response {
            typ: "echo_ok".into(),
            src: None,
            dest: None,
            body: r.body,
        }
    }
}

fn main() {
    pretty_env_logger::init_timed();

    let app = App {
        context: Arc::new(RwLock::new(None)),
    };

    let runtime = new_runtime(4, 36);

    runtime.spawn(async {
        let r = request(Request {
            src: None,
            dest: Some("node1".into()),
            typ: "hello".into(),
            body: None,
        });
        println!("got response: {:?}", r);
    });

    runtime.block_on(listen(move |x| {
        let app = app.clone();
        app.handler(x)
    }));
}
