use core::time;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::{
    proto::{Request, Response},
    App,
};

pub async fn handle(app: &mut App, r: &Request) -> Response {
    // generate a 64 bit snowflake ID
    // https://en.wikipedia.org/wiki/Snowflake_ID

    let node_id = {
        let lock = app.context.read().unwrap();
        let node_id = lock.as_ref().unwrap().node_id.as_str();

        node_id
            .chars()
            .skip(1)
            .collect::<String>()
            .parse::<u128>()
            .unwrap()
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // increment and get the sequence if the timestamp is the same, otherwise
    // reset the sequence
    let sequence = {
        let mut lock = app.unique_id_sequence.write().unwrap();
        if lock.0 == timestamp {
            lock.1 += 1;
        } else {
            lock.0 = timestamp;
            lock.1 = 1;
        }

        lock.1
    };

    let id = (timestamp << 23) | (node_id << 13) | sequence as u128;

    Response::new("generate_ok").with_body(json!({ "id": id }).as_object().unwrap().to_owned())
}
