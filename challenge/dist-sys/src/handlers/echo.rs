use crate::{
    proto::{Request, Response},
    App,
};

pub async fn handle(_: &mut App, r: &Request) -> Response {
    Response::new("echo_ok").with_body(r.body.clone().unwrap())
}
