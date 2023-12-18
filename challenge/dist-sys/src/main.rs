use std::{
    io::{self, Write},
    rc::Rc,
    sync::mpsc,
    thread,
};

use serde_json::{Map, Value};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Message {
    src: String,
    dest: String,
    body: Body,
}

#[derive(Clone)]
struct Context {
    send: mpsc::Sender<Message>,
    node_id: Rc<String>,
    node_ids: Rc<Vec<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Body {
    #[serde(rename = "type")]
    typ: String,
    msg_id: Option<i64>,
    in_reply_to: Option<i64>,

    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl From<anyhow::Error> for Message {
    fn from(value: anyhow::Error) -> Self {
        Message {
            src: "TODO".into(),
            dest: "TODO".into(),
            body: Body {
                typ: "error".into(),
                msg_id: None,
                in_reply_to: None,
                extra: Map::from_iter([("error".to_string(), Value::String(value.to_string()))]),
            },
        }
    }
}

fn main() {
    let mut buffer = String::new();

    io::stdin().read_line(&mut buffer).unwrap();
    let message: Message = serde_json::from_str(&buffer).unwrap();
    if message.body.typ != "init" {
        panic!("first message must be init");
    }

    // use this to send extra messages to the coordinator apart from the
    // main thread which handles the RPCs
    let (send_response, recv_response) = mpsc::channel();

    thread::spawn(move || {
        for msg in recv_response {
            let response_str = serde_json::to_string(&msg).unwrap() + "\n";
            io::stdout().write_all(response_str.as_bytes()).unwrap();
        }
    });

    let context = Context {
        send: send_response,
        node_id: Rc::new(
            message
                .body
                .extra
                .get("node_id")
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
        ),
        node_ids: Rc::new(
            message
                .body
                .extra
                .get("node_ids")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
        ),
    };

    buffer.clear();

    while let Ok(length) = io::stdin().read_line(&mut buffer) {
        let message: Message = serde_json::from_str(&buffer).unwrap();

        let response = match message.body.typ.as_str() {
            "echo" => handle_echo(&message, context.clone()),
            _ => panic!("invalid message type"),
        };

        let res = match response {
            Ok(r) => r,
            Err(e) => Message::from(e),
        };

        let response_str = serde_json::to_string(&res).unwrap() + "\n";
        io::stdout().write_all(response_str.as_bytes()).unwrap();

        buffer.clear();
    }
}

fn handle_echo(msg: &Message, context: Context) -> Result<Message, anyhow::Error> {
    let echo = msg
        .body
        .extra
        .get("echo")
        .ok_or(anyhow::anyhow!(
            "message is type echo but no echo field present"
        ))?
        .clone();

    let response = Message {
        src: "n1".into(),
        dest: "c1".into(),
        body: Body {
            typ: "echo_ok".into(),
            msg_id: Some(1),
            in_reply_to: msg.body.msg_id,
            extra: Map::from_iter([("echo".to_string(), echo)]),
        },
    };

    Ok(response)
}
