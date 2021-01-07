mod cli;
mod common;
//mod ui;

pub mod symbols {
    pub use crate::cli::*;
    pub use crate::common::*;
    //pub use crate::ui::*;
}

pub mod imports {
    pub use chrono::{DateTime, Utc};
    pub use crossbeam::channel;
    pub use futures::{SinkExt, Stream, StreamExt, TryFutureExt};
    pub use hashbrown::HashMap;

    pub use log::{debug, error, info, warn};
    pub use rand::{rngs::ThreadRng, Rng};
    pub use serde::{Deserialize, Serialize};
    pub use std::{
        error::Error,
        fmt::Display,
        path::{Path, PathBuf},
        pin::Pin,
        sync::Arc,
        time::{Duration, Instant},
    };
    pub use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    pub use uuid::Uuid;
}

use crate::imports::*;
use crate::symbols::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    const HTTP_ADDR: &'static str = "http://127.0.0.1:8080/";
    const WS_ADDR: &'static str = "ws://127.0.0.1:9999/";
    const EMAIL: &'static str = "a@a.com";
    const PHASH: &'static str = "blah";
    const PUBKEY: &'static str = "bleh";
    let args: Vec<String> = std::env::args().collect();
    debug!("args {:?}", &args);
    // register?
    if args[6].parse() == Ok(true) {
        register(&args[1], &args[3], &args[4], &args[5]).await;
    }
    connect(&args[1], &args[2], &args[3], &args[4]).await;
}

async fn register(
    http_addr: &str,
    email: &str,
    phash: &str,
    pubkey: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    client
        .post(&format!("{}{}", http_addr, "register"))
        .body(
            serde_json::to_string(&RegisterRequest {
                email: email.to_owned(),
                password_hash: phash.to_owned(),
                pubkey: pubkey.to_owned(),
            })
            .unwrap(),
        )
        .send()
        .await
}

async fn connect(http_addr: &str, ws_addr: &str, email: &str, phash: &str) {
    let client = reqwest::Client::new();
    /*let reg_res = client.post(&format!("{}{}", HTTP_ADDR, "register"))
    .body(serde_json::to_string(&RegisterRequest {
        email: EMAIL.to_owned(),
        password_hash: PHASH.to_owned(),
        pubkey: PUBKEY.to_owned()
    }).unwrap()).send().await;
    println!("{:?}", &reg_res);*/
    let lt = client
        .post(&format!("{}{}", http_addr, "login"))
        .body(
            serde_json::to_string(&LoginRequest {
                email: email.to_owned(),
                password_hash: phash.to_owned(),
            })
            .unwrap(),
        )
        .send()
        .await
        .unwrap()
        .json::<LoginToken>()
        .await
        .unwrap();
    println!("{:?}", &lt);

    tokio::time::delay_for(Duration::from_millis(200)).await;
    //let (mut ctrlc_s, mut ctrlc_r) = tokio::sync::mpsc::channel(1);
    /*ctrlc::set_handler(move || {
        ctrlc_s.send(());
    })
    .expect("Error setting Ctrl-C handler");*/
    let (mut wss, resp) = tokio_tungstenite::connect_async(
        http::request::Request::builder()
            .uri(ws_addr)
            .header("Authorization", &lt.tk)
            .body(())
            .unwrap(),
    )
    .await
    .unwrap();
    info!("connected to {}", &ws_addr);
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut run = true;
    let mut dm_dest = None;
    let mut state = ClientState::Connected;
    while run {
        tokio::select! {
            maybe_ws = wss.next() => {
                match maybe_ws {
                    Some(raw_ws_inc) => {
                        debug!("ws raw inc {:?}", &raw_ws_inc);
                        if let Ok(tungstenite::Message::Text(m_ws_clientbound)) = raw_ws_inc {
                            if let Ok(wsc) = serde_json::from_str::<WsClientboundPayload>(&m_ws_clientbound) {
                                debug!("ws inc decoded: {:?}", &wsc);
                                match wsc {
                                    WsClientboundPayload::NewMessage(m) => {
                                        info!("({}) <<< {}", m.from, m.content);
                                    },
                                    _ => {}
                                }
                            }
                        } else {
                            warn!("ws unknown inc");
                        }
                    },
                    None => {
                        info!("Server closed\n");
                        run = false;
                    }
                }
            }
            Some(Ok(ln)) = lines.next() => {
                debug!("> {}", &ln);
                match parse(&ln, state.clone()) {
                    Ok(cmd) => match cmd {
                        CliCommand::SelectGroup(gid) => {},
                        CliCommand::SelectUser(uid) => {
                            info!("(dm) Targeting {}", &uid);
                            dm_dest = Some(uid);
                        },
                        CliCommand::Text(s) => {
                            match dm_dest {
                                Some(uid) => {
                                    wss.send(WsServerboundPayload::NewUserMessage {
                                        to: uid,
                                        content: ClientMessage::from(s)
                                    }.into()).await;
                                },
                                None => {
                                    warn!("Missing recipient");
                                }
                            }
                        }
                        _ => {
                            warn!("Command not recognized/implemented yet");
                        }
                    },
                    Err(e) => {
                        print_parse_e(e);
                    }
                }
            }
            /*Some(_) = ctrlc_r.next() => {
                info!("shutting down");
                run = false;
            }*/
        }
    }
}

fn print_parse_e(e: CliParseError) {
    match e {
        CliParseError::CannotChatNow(_) => {
            error!("Cannot chat right now");
        }
        CliParseError::Empty => {
            warn!("Empty line");
        }
        CliParseError::NotAscii => {
            error!("Unexpected non-ascii characters");
        }
        CliParseError::UnrecognizedCommand(c) => {
            error!("Unrecognized command {}", &c);
        }
        CliParseError::TypeError(id) => {
            error!("Invalid type {:?}", id);
        }
        CliParseError::MissingExpected(v) => {
            error!("EOL but expected {}", &v);
        }
        CliParseError::NotImpl => {
            warn!("Command not implemented");
        }
    }
}
