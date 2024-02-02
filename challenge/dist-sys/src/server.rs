use futures::{join, Future};

use crate::{
    client::request,
    proto::{RPC, RPC_INSTANCE},
    Request, Response,
};

pub async fn listen<F, R>(handler: F)
where
    F: FnMut(Request) -> R + Send + Clone + 'static,
    R: Future<Output = Response> + Send,
{
    let rpc = unsafe {
        let _ = RPC_INSTANCE.set(RPC::new());
        RPC_INSTANCE.get_mut().unwrap()
    };

    rpc.listen(handler).await;
}
