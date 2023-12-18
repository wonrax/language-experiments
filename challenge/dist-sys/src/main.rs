use std::{
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
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
    send: mpsc::Sender<Request>,
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

struct Request {
    msg: Message,

    // Use this to send a response back to the client
    send: mpsc::Sender<Message>,
}

fn main() {
    let mut buffer = String::new();

    io::stdin().read_line(&mut buffer).unwrap();
    let message: Message = serde_json::from_str(&buffer).unwrap();
    if message.body.typ != "init" {
        panic!("first message must be init");
    }

    // use this as a RPC client, which is different than the main loop below
    // which acts like a listen server
    let (send_request, recv_request) = mpsc::channel::<Request>();

    let mut request_senders: Arc<Mutex<HashMap<i64, mpsc::Sender<Message>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let request_senders_clone = request_senders.clone();

    thread::spawn(move || {
        for request in recv_request {
            let request_str = serde_json::to_string(&request.msg).unwrap() + "\n";
            io::stdout().write_all(request_str.as_bytes()).unwrap();

            let mut request_senders = request_senders_clone.lock().unwrap();
            request_senders.insert(request.msg.body.msg_id.unwrap(), request.send);
        }
    });

    let context = Context {
        send: send_request,
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

    let request_senders_clone = request_senders.clone();
    let response_buffer = String::new();

    thread::spawn(|| {});

    while let Ok(length) = io::stdin().read_line(&mut buffer) {
        let message: Message = serde_json::from_str(&buffer).unwrap();

        let response = match message.body.typ.as_str() {
            "echo" => handle_echo(&message, context.clone()).map(|r| Some(r)),
            _ => Ok(None),
        };

        let res = match response {
            Ok(r) => r,
            Err(e) => Some(Message::from(e)),
        };

        if let Some(res) = res {
            let response_str = serde_json::to_string(&res).unwrap() + "\n";
            io::stdout().write_all(response_str.as_bytes()).unwrap();
        }

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

    let (message_send, message_recv) = mpsc::channel::<Message>();

    context.send.send(Request {
        msg: Message {
            src: "node".into(),
            dest: "me".into(),
            body: Body {
                typ: "echo".into(),
                msg_id: Some(69),
                in_reply_to: None,
                extra: Map::new(),
            },
        },
        send: message_send,
    });

    for message in message_recv {
        println!("got message: {:?}", message);
    }

    Ok(response)
}
