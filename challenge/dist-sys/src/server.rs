use futures::Future;

use crate::{
    proto::{RPC, RPC_INSTANCE},
    Request, Response,
};

pub async fn listen<F, R>(handler: F)
where
    F: Fn(Request) -> R + Send + Copy + 'static,
    R: Future<Output = Response> + Send,
{
    let rpc = unsafe {
        let _ = RPC_INSTANCE.set(RPC::new());
        RPC_INSTANCE.get_mut().unwrap()
    };

    rpc.listen(handler).await;
}
