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
            lock.push(number);
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
            // TODO
            Response::new("topology_ok")
        }
        _ => unreachable!(),
    }
}
