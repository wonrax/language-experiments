use std::{
    collections::HashMap,
    io::{self, Write},
};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Message {
    src: String,
    dest: String,
    body: EchoBodyResponse,
}

#[derive(serde::Deserialize, Debug)]
struct EchoBody {
    r#type: String,
    msg_id: i64,
    echo: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct EchoBodyResponse {
    r#type: String,
    msg_id: i64,
    in_reply_to: Option<i64>,
    echo: String,
}

fn main() {
    let mut buffer = String::new();
    while let Ok(length) = io::stdin().read_line(&mut buffer) {
        let message: Message = serde_json::from_str(&buffer).unwrap();

        let response = Message {
            src: "n1".into(),
            dest: "c1".into(),
            body: EchoBodyResponse {
                r#type: "echo".into(),
                msg_id: 1,
                in_reply_to: Some(message.body.msg_id),
                echo: message.body.echo,
            },
        };

        let response_str = serde_json::to_string(&response).unwrap();

        // this doesnt work
        // io::stdout().write_all(response_str.as_bytes()).unwrap();

        println!("{}", response_str);

        buffer.clear();
    }
}
