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
    PROTOCOL
        .set(Protocol::new())
        .expect("Protocol should not be initialized yet.");

    PROTOCOL
        .get()
        .expect(
            "Protocol should've been initialized by now. \
                            Either the protocol is not initialized or \
                            you're trying to use the protocol from a \
                            different thread.",
        )
        .listen(handler)
        .await;
}
