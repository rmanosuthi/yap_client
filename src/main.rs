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
    pub use futures::{Stream, StreamExt, SinkExt, TryFutureExt};
    pub use hashbrown::HashMap;

    pub use log::{debug, error, info, warn};
    pub use rand::{rngs::ThreadRng, Rng};
    pub use serde::{Deserialize, Serialize};
    pub use std::{
        fmt::Display,
        error::Error,
        path::{Path, PathBuf},
        pin::Pin,
        sync::Arc,
        time::{Duration, Instant},
    };
    pub use uuid::Uuid;
    pub use tokio::{io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt}};
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

async fn register(http_addr: &str, email: &str, phash: &str, pubkey: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    client.post(&format!("{}{}", http_addr, "register"))
    .body(serde_json::to_string(&RegisterRequest {
        email: email.to_owned(),
        password_hash: phash.to_owned(),
        pubkey: pubkey.to_owned()
    }).unwrap()).send().await
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
    let lt = client.post(&format!("{}{}", http_addr, "login"))
    .body(serde_json::to_string(&LoginRequest {
        email: email.to_owned(),
        password_hash: phash.to_owned()
    }).unwrap()).send().await.unwrap().json::<LoginToken>().await.unwrap();
    println!("{:?}", &lt);

    tokio::time::delay_for(Duration::from_millis(200)).await;
    //let (mut ctrlc_s, mut ctrlc_r) = tokio::sync::mpsc::channel(1);
    /*ctrlc::set_handler(move || {
        ctrlc_s.send(());
    })
    .expect("Error setting Ctrl-C handler");*/
    let (mut wss, resp) = tokio_tungstenite::connect_async(
        http::request::Request::builder().uri(ws_addr).header("Authorization", &lt.tk).body(()).unwrap()
    ).await.unwrap();
    info!("connected to {}", &ws_addr);
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut run = true;
    let mut dm_dest = None;
    const PFX_DM: &str = "dm ";
    while run {
        tokio::select! {
            Some(raw_ws_inc) = wss.next() => {
                debug!("ws raw inc {:?}", &raw_ws_inc);
                if let Ok(tungstenite::Message::Text(m_ws_clientbound)) = raw_ws_inc {
                    if let Ok(wsc) = serde_json::from_str::<WsClientbound>(&m_ws_clientbound) {
                        debug!("ws inc decoded: {:?}", &wsc);
                        match wsc {
                            WsClientbound::NewUserMessage {from, c, umid: _} => {
                                info!("({}) <<< {}", from, c);
                            },
                            _ => {}
                        }
                    }
                } else {
                    warn!("ws unknown inc");
                }
            }
            Some(Ok(ln)) = lines.next() => {
                debug!("> {}", &ln);
                if ln.starts_with(PFX_DM) {
                    if let Ok(dest_uid) = (&ln[PFX_DM.len()..]).parse::<u32>() {
                        info!("setting dm target to {}", dest_uid);
                        dm_dest = Some(dest_uid);
                    } else {
                        error!("dm - invalid uid");
                    }
                } else {
                    if let Some(uid) = dm_dest {
                        wss.send(tungstenite::Message::Text(
                            serde_json::to_string(
                                &WsServerbound::NewUserMessage {
                                    to: uid.into(),
                                    c: ln.clone().into()
                                }
                            ).unwrap()
                        )).await;
                        info!("(self) >>> {}", ln);
                    } else {
                        error!("missing dm target");
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