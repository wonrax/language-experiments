use async_runtime::runtime::current;
use serde_json::json;

use crate::{
    proto::{Request, Response},
    App,
};

pub async fn handle(app: &mut App, r: &Request) -> Response {
    match r.typ.as_str() {
        "broadcast" => {
            let number = r.body.as_ref().unwrap()["message"].as_i64().unwrap();

            if app.broadcast_data.read().unwrap().contains(&number) {
                return Response::new("broadcast_ok");
            }

            let mut lock = app.broadcast_data.write().unwrap();
            lock.insert(number);
            drop(lock);

            // broadcast to all nodes
            let topo_lock = app.topology.read().unwrap();
            let ctx_lock = app.context.read().unwrap();

            let this_node_id = &ctx_lock.as_ref().unwrap().node_id;

            for adj_node in topo_lock.get(this_node_id).unwrap() {
                let mut r = r.clone();
                r.dest = Some(adj_node.clone());
                r.src = Some(this_node_id.clone());

                current().spawn(async move {
                    loop {
                        // TODO implement backoff
                        let res = crate::client::request(r.clone()).await;
                        if res.typ == "broadcast_ok" {
                            break;
                        }
                    }
                });
            }

            Response::new("broadcast_ok")
        }
        "read" => {
            let lock = app.broadcast_data.read().unwrap();
            Response::new("read_ok").with_body(
                json!({
                    "messages": *lock
                })
                .as_object()
                .unwrap()
                .to_owned(),
            )
        }
        "topology" => {
            let mut lock = app.topology.write().unwrap();
            let topo = r.body.as_ref().unwrap()["topology"].as_object().unwrap();
            for (k, v) in topo {
                let mut nodes = Vec::new();
                for node in v.as_array().unwrap() {
                    nodes.push(node.as_str().unwrap().to_string());
                }
                lock.insert(k.to_string(), nodes);
            }
            Response::new("topology_ok")
        }
        _ => unreachable!(),
    }
}
