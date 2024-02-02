mod client;
mod handlers;
mod proto;
mod server;
mod stdin;
mod timer;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use async_runtime::runtime;
use proto::{Request, Response};
use serde_json::json;

struct Inner {
    context: RwLock<Option<Context>>,
    unique_id_sequence: RwLock<(u128, u32)>,
    broadcast_data: RwLock<HashSet<i64>>,
    topology: RwLock<HashMap<String, Vec<String>>>,
}

type App = Arc<Inner>;

#[derive(Debug)]
struct Context {
    node_id: String,
    node_ids: Vec<String>,
}

// root handler aka router that dispatches messages to the appropriate handler
async fn main_handler(mut app: App, r: Request) -> Response {
    // check if the dest is this node
    {
        let ctx = app.context.read().unwrap();
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

    // TODO handle unknown message types by returning error message
    let mut response = match r.typ.as_str() {
        "init" => handlers::init::handle(&mut app, &r).await,
        "echo" => handlers::echo::handle(&mut app, &r).await,
        "generate" => handlers::unique_id::handle(&mut app, &r).await,
        "broadcast" | "read" | "topology" => handlers::broadcast::handle(&mut app, &r).await,
        _ => panic!("unknown message type: request: {:?}", r),
    };

    response.src = Some(
        app.context
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

fn main() {
    pretty_env_logger::init_timed();

    let app = Arc::new(Inner {
        context: RwLock::new(None),
        unique_id_sequence: RwLock::new((0, 0)),
        broadcast_data: RwLock::new(HashSet::new()),
        topology: RwLock::new(HashMap::new()),
    });

    let runtime = runtime::new_runtime(4, 36);

    runtime.block_on(server::listen(move |x| main_handler(app.clone(), x)));
}
