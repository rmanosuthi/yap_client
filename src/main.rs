mod auth;
mod cli;
mod common;

#[macro_use]
extern crate structopt;

pub mod symbols {
    pub use crate::auth::*;
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

use log::LevelFilter;
use openssl::rsa::Rsa;
use sha2::{Digest, Sha256};
use std::{
    convert::TryFrom,
    fs::File,
    io::{BufReader, BufWriter},
};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();
    let opt = LaunchOptions::from_args();
    match opt {
        LaunchOptions::Login { cfg_path } => {
            let f = File::open(&cfg_path)?;
            let mut buf = BufReader::new(f);
            let cfg = serde_json::from_reader(buf)?;
            info!("Loaded config file");
            connect(cfg).await;
        }
        LaunchOptions::Register {
            save_to,
            http_addr,
            email,
            password,
        } => {
            let client = reqwest::Client::new();
            // generate password hash
            let phash = hex::encode({
                let mut hasher = Sha256::new();
                hasher.update(password.as_bytes());
                hasher.finalize().to_vec()
            });
            debug!("Register: derived phash {}", &phash);
            let privkey = Rsa::generate(2048)?;
            let privkey_serialized = String::from_utf8(privkey.private_key_to_pem().unwrap())?;
            let pubkey_serialized = String::from_utf8(privkey.public_key_to_pem().unwrap())?;
            let local_ident = LocalIdentity {
                privkey: privkey_serialized.clone(),
                pubkey: pubkey_serialized.clone(),
            };
            match client
                .post(&format!("{}{}", http_addr, "register"))
                .body(
                    serde_json::to_string(&RegisterRequest {
                        email: email.to_owned(),
                        password_hash: phash.to_owned(),
                        pubkey: pubkey_serialized.to_owned(),
                    })
                    .unwrap(),
                )
                .send()
                .await
            {
                Ok(_) => {
                    info!("Registered.");
                    let gen_cfg = LocalServerEntry {
                        http_addr: http_addr,
                        ws_addr: "".to_owned(),
                        email: email,
                        phash: phash,
                        identity: local_ident,
                    };
                    // sync code
                    let f = File::create(&save_to)?;
                    let buf = BufWriter::new(f);
                    serde_json::to_writer_pretty(buf, &gen_cfg).unwrap();
                    info!(
                        "Written config to {:?}, make sure to edit the ws address!",
                        &save_to
                    );
                }
                Err(e) => {
                    error!("Failed to register: {:?}", e);
                }
            }
        }
    }
    Ok(())
}

#[derive(StructOpt)]
pub enum LaunchOptions {
    Login {
        #[structopt(parse(from_os_str))]
        cfg_path: PathBuf,
    },
    Register {
        #[structopt(parse(from_os_str))]
        save_to: PathBuf,
        http_addr: String,
        email: String,
        password: String,
    },
}

async fn connect(cfg: LocalServerEntry) {
    let client = reqwest::Client::new();
    let lt = client
        .post(&format!("{}{}", cfg.http_addr, "login"))
        .body(
            serde_json::to_string(&LoginRequest {
                email: cfg.email.to_owned(),
                password_hash: cfg.phash.to_owned(),
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
    let (mut wss, resp) = tokio_tungstenite::connect_async(
        http::request::Request::builder()
            .uri(&cfg.ws_addr)
            .header("Authorization", &lt.tk)
            .body(())
            .unwrap(),
    )
    .await
    .unwrap();
    info!("connected to {}", &cfg.ws_addr);
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut run = true;
    let mut dm_dest = None;
    let mut state = ClientState::Connected;
    let mut cache_users: HashMap<UserId, PublicUserRecord> = HashMap::new();
    let key = InMemoryKey::try_from(cfg.identity.clone()).unwrap();
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
                                        let uid = &m.from;
                                        // try fetch user data
                                        if let Some(pur) = cache_users.get(uid) {
                                            if let Some(dec) = pur.decrypt(m.content) {
                                                info!("(decrypted, {}) <<< {}", m.from, dec);
                                            } else {
                                                error!("Failed to decrypt incoming message");
                                            }
                                        } else {
                                            match get_user(&cfg, &client, uid).await {
                                                Ok(pur) => {
                                                    cache_users.insert(uid.to_owned(), pur.clone());
                                                    info!("Added user cache {}", uid);
                                                    if let Some(dec) = pur.decrypt(m.content) {
                                                        info!("(decrypted, {}) <<< {}", m.from, dec);
                                                    } else {
                                                        error!("Failed to decrypt incoming message");
                                                    }
                                                },
                                                Err(e) => {
                                                    error!("Failed to get user {}: {:?}", uid, e);
                                                }
                                            }
                                        }
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
                            if let Some(pur) = cache_users.get(&uid) {
                                // user exists and is cached

                            info!("(dm) Targeting {}", &uid);
                                dm_dest = Some(uid);
                            } else {
                                // user isn't cached, ask server
                                match get_user(&cfg, &client, &uid).await {
                                    Ok(pur) => {
                                        cache_users.insert(uid, pur);
                                        info!("(dm) Fetched data, targeting {}", &uid);
                                        dm_dest = Some(uid);
                                    },
                                    Err(e) => {
                                        error!("Failed to get user {}: {:?}", uid, e);
                                    }
                                }
                            }
                        },
                        CliCommand::Text(s) => {
                            match dm_dest {
                                Some(uid) => {
                                    if let Some(enc) = key.encrypt(&s) {
                                        wss.send(WsServerboundPayload::NewUserMessage {
                                            to: uid,
                                            content: ClientMessage::from(enc)
                                        }.into()).await;
                                        info!("(encrypted, self) >>> {}", s);
                                    } else {
                                        error!("Failed to encrypt message");
                                    }
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

#[derive(Debug)]
pub enum GetUserError {
    RequestFailed,
    DeserializeFailed,
}

async fn get_user(
    cfg: &LocalServerEntry,
    client: &reqwest::Client,
    uid: &UserId,
) -> Result<PublicUserRecord, GetUserError> {
    let resp = client
        .get(&format!("{}{}/{}", cfg.http_addr, "users", uid))
        .send()
        .await
        .map_err(|_| GetUserError::RequestFailed)?;
    resp.json()
        .map_err(|_| GetUserError::DeserializeFailed)
        .await
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
