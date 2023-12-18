use std::{
    collections::HashMap,
    io::{self, Write},
};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Message {
    src: String,
    dest: String,
    body: Body,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct BodyCommon {
    msg_id: Option<i64>,
    in_reply_to: Option<i64>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type")]
enum Body {
    #[serde(rename = "echo")]
    Echo {
        #[serde(flatten)]
        common: BodyCommon,
        echo: String,
    },
    #[serde(rename = "echo_ok")]
    EchoOk {
        #[serde(flatten)]
        common: BodyCommon,
        echo: String,
    },
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

        println!("message {:#?}", message);

        match message.body {
            Body::Echo { common, echo } => {
                let response = Message {
                    src: "n1".into(),
                    dest: "c1".into(),
                    body: Body::EchoOk {
                        common: BodyCommon {
                            msg_id: Some(1),
                            in_reply_to: common.msg_id,
                        },
                        echo: echo,
                    },
                };

                let response_str = serde_json::to_string(&response).unwrap() + "\n";

                io::stdout().write_all(response_str.as_bytes()).unwrap();
            }
            _ => {}
        }

        buffer.clear();
    }
}
