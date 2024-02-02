use futures::{join, Future};

use crate::{
    client::request,
    proto::{Protocol, PROTOCOL},
    Request, Response,
};

pub async fn listen<F, R>(handler: F)
where
    F: FnMut(Request) -> R + Send + Clone + 'static,
    R: Future<Output = Response> + Send,
{
    let rpc = unsafe {
        let _ = PROTOCOL.set(Protocol::new());
        PROTOCOL.get_mut().unwrap()
    };

    rpc.listen(handler).await;
}
