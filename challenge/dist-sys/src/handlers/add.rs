use crate::{
    proto::{Request, Response},
    App,
};

pub async fn handle(app: &mut App, r: &Request) -> Response {
    match r.typ.as_str() {
        "add" => {
            let number = r.body.as_ref().unwrap()["delta"].as_u64().unwrap();
            let mut lock = app.counter.write().unwrap();
            *lock += number;
            Response::new("add_ok")
        }
        "read" => unimplemented!(),
        _ => unreachable!(),
    }
}
