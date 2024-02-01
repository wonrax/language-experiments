mod client;
mod proto;
mod server;
mod stdin;
mod timer;

use std::sync::{Arc, RwLock};

use async_runtime::runtime;
use client::request;
use log::debug;
use proto::{Request, Response};
use serde_json::{json, Map};
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
    async fn main_handler(mut self, r: Request) -> Response {
        {
            let ctx = self.context.read().unwrap();
            if let Some(ctx) = ctx.as_ref() {
                if &ctx.node_id != r.dest.as_ref().unwrap() {
                    let mut response = Response::new("err").with_body(
                        json!({
                            "msg": "invalid dest"
                        })
                        .as_object()
                        .unwrap()
                        .to_owned(),
                    );

                    response.src = Some(ctx.node_id.clone());
                    response.dest = r.src;

                    return response;
                }
            }
        }

        let mut response = match r.typ.as_str() {
            "init" => {
                let mut context = self.context.write().unwrap();
                if context.is_some() {
                    panic!("context already initialized");
                }
                let node_id = r.body.as_ref().unwrap()["node_id"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let node_ids: Vec<String> = r.body.as_ref().unwrap()["node_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap().to_string())
                    .collect();
                *context = Some(Context { node_id, node_ids });
                debug!("initialized context: {:?}", context);
                Response::new("init_ok").with_body(r.body.clone().unwrap())
            }
            "echo" => self.echo_handler(&r).await,
            _ => panic!("unknown message type: {}", r.typ),
        };

        response.src = Some(
            self.context
                .read()
                .unwrap()
                .as_ref()
                .expect("context not initialized, an init message should be sent first")
                .node_id
                .clone(),
        );
        response.dest = Some(r.src.clone().unwrap());

        response
    }

    async fn echo_handler(&mut self, r: &Request) -> Response {
        Response::new("echo_ok").with_body(r.body.clone().unwrap())
    }
}

fn main() {
    pretty_env_logger::init_timed();

    let app = App {
        context: Arc::new(RwLock::new(None)),
    };

    let runtime = runtime::new_runtime(4, 36);

    // runtime.spawn(async {
    //     let r = request(Request {
    //         src: None,
    //         dest: Some("node1".into()),
    //         typ: "hello".into(),
    //         body: None,
    //     });
    //     println!("got response: {:?}", r);
    // });

    runtime.block_on(listen(move |x| {
        let app = app.clone();
        app.main_handler(x)
    }));
}
