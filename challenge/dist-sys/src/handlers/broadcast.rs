use serde_json::json;

use crate::{
    proto::{Request, Response},
    App,
};

pub async fn handle(app: &mut App, r: &Request) -> Response {
    match r.typ.as_str() {
        "broadcast" => {
            let number = r.body.as_ref().unwrap()["message"].as_i64().unwrap();
            let mut lock = app.broadcast_data.write().unwrap();
            lock.insert(number);
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
