use log::debug;

use crate::{
    proto::{Request, Response},
    App, Context,
};

pub async fn handle(app: &mut App, r: &Request) -> Response {
    let mut context = app.context.write().unwrap();
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

    Response::new("init_ok")
}
