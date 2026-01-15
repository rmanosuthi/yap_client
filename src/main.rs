use anyhow::{Context, Error, Result, anyhow};
use std::{collections::HashMap, path::PathBuf, pin::pin, sync::Arc, time::Duration};

use clap::{Parser, Subcommand};
use common::{
    ClientMessage, LoginRequest, LoginToken, PublicUserRecord, RegisterRequest, UserId,
    WsClientboundPayload, WsServerboundPayload,
};
use futures::{SinkExt, StreamExt, TryFutureExt, stream::unfold};
use log::{LevelFilter, debug, error, info, warn};
use openssl::{rsa::Rsa, sha::Sha256};
use tokio::{
    io::{AsyncBufReadExt, stdin},
    sync::Mutex,
    time,
};
use tungstenite::{client::IntoClientRequest, http::Request};

use crate::{
    auth::{InMemoryKey, LocalIdentity, LocalServerEntry},
    cli::{CliCommand, CliParseError, ClientState, parse},
};

mod auth;
mod cli;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct LaunchConfig {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login {
        #[arg(short, long)]
        cfg_path: PathBuf,
    },
    Register {
        #[arg(short, long)]
        cfg_path: PathBuf,
        #[arg(short, long)]
        addr: String,
        #[arg(short, long)]
        email: String,
        #[arg(short, long)]
        password: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();
    let opt = LaunchConfig::parse();
    match opt.command {
        Commands::Login { cfg_path: cfg_path } => {
            let f = std::fs::File::open(&cfg_path)?;
            let mut buf = std::io::BufReader::new(f);
            let cfg = serde_json::from_reader(buf.into_inner())?;
            info!("Loaded config file");
            connect(cfg).await.map_err(|e| {
                error!("Connection failed: {}", e);
                e
            })
        }
        Commands::Register {
            cfg_path: cfg_path,
            addr,
            email,
            password,
        } => {
            let client = reqwest::Client::new();
            // generate password hash
            let phash = hex::encode({
                let mut hasher = Sha256::new();
                hasher.update(password.as_bytes());
                hasher.finish().to_vec()
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
                .post(format!("{}/{}", addr, "register"))
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
                        http_addr: addr,
                        email: email,
                        phash: phash,
                        identity: local_ident,
                    };
                    // sync code
                    let f = std::fs::File::create(&cfg_path)?;
                    let buf = std::io::BufWriter::new(f);
                    serde_json::to_writer_pretty(buf, &gen_cfg).unwrap();
                    info!("Written config to {}", cfg_path.display());
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to register: {:?}", e);
                    return Err(e.into());
                }
            }
        }
    }
}

enum Event {
    IncomingRawWs(Result<tungstenite::Message>),
    ConsoleLine(String),
    ConsoleError(Error),
    ConsoleEmptyLine,
    ServerClosed,
}

async fn connect(cfg: LocalServerEntry) -> Result<()> {
    let client = reqwest::Client::new();
    let lt_resp = client
        .post(format!("{}/{}", cfg.http_addr, "login"))
        .body(
            serde_json::to_string(&LoginRequest {
                email: cfg.email.to_owned(),
                password_hash: cfg.phash.to_owned(),
            })
            .unwrap(),
        )
        .send()
        .await?;

    let ws_addr = lt_resp
        .headers()
        .get("Yap-Ws-Url")
        .ok_or(anyhow!("Server response is missing Yap-Ws-Url"))
        .map(|hdr_val| hdr_val.to_str())??
        .to_owned();
    debug!("ws addr {}", &ws_addr);
    let lt = lt_resp.json::<LoginToken>().await?;
    debug!("lt {:#?}", &lt);

    let mut req_wss_init = format!("ws://{}", &ws_addr)
        .into_client_request()
        .with_context(|| "Failed to build request from websocket address")?;
    req_wss_init
        .headers_mut()
        .insert("Authorization", lt.tk.parse().unwrap());
    let (wss, resp) = tokio_tungstenite::connect_async(req_wss_init)
        .await
        .with_context(|| "Failed to establish websocket connection")?;
    let (mut wss_w, mut wss_r) = wss.split();
    info!("connected to {}", &ws_addr);
    let mut lines = tokio::io::BufReader::new(stdin()).lines();
    let mut run = true;
    let mut dm_dest = None;
    let mut state = ClientState::Connected;
    let mut cache_users: HashMap<UserId, PublicUserRecord> = HashMap::new();
    let key = InMemoryKey::try_from(cfg.identity.clone()).unwrap();
    let ev_incoming_wss = unfold(wss_r, |mut wss_r| async {
        match wss_r.next().await {
            Some(raw_ws_inc) => Some((
                Event::IncomingRawWs(raw_ws_inc.map_err(|e| e.into())),
                wss_r,
            )),
            None => Some((Event::ServerClosed, wss_r)),
        }
    });
    let ev_incoming_line = unfold(lines, |mut lines| async {
        Some((
            match lines.next_line().await {
                Ok(Some(ln)) => Event::ConsoleLine(ln),
                Ok(None) => Event::ConsoleEmptyLine,
                Err(e) => Event::ConsoleError(e.into()),
            },
            lines,
        ))
    });
    let mut evs = pin!(tokio_stream::StreamExt::merge(ev_incoming_wss, ev_incoming_line));
    while let Some(ev) = evs.next().await {
        match ev {
            Event::IncomingRawWs(raw_ws_inc) => {
                debug!("ws raw inc {:?}", &raw_ws_inc);
                if let Ok(tungstenite::Message::Text(m_ws_clientbound)) = raw_ws_inc {
                    if let Ok(wsc) = serde_json::from_str::<WsClientboundPayload>(&m_ws_clientbound)
                    {
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
                                        }
                                        Err(e) => {
                                            error!("Failed to get user {}: {:?}", uid, e);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    warn!("ws unknown inc");
                }
            }
            Event::ServerClosed => {
                info!("Server closed\n");
                break;
            }
            Event::ConsoleLine(ln) => {
                debug!("> {}", &ln);
                match parse(&ln, state.clone()) {
                    Ok(cmd) => match cmd {
                        CliCommand::SelectGroup(gid) => {}
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
                                    }
                                    Err(e) => {
                                        error!("Failed to get user {}: {:?}", uid, e);
                                    }
                                }
                            }
                        }
                        CliCommand::Text(s) => match dm_dest {
                            Some(uid) => {
                                if let Some(enc) = key.encrypt(&s) {
                                    wss_w
                                        .send(
                                            WsServerboundPayload::NewUserMessage {
                                                to: uid,
                                                content: enc,
                                            }
                                            .into(),
                                        )
                                        .await?;
                                    info!("(encrypted, self) >>> {}", s);
                                } else {
                                    error!("Failed to encrypt message");
                                }
                            }
                            None => {
                                warn!("Missing recipient");
                            }
                        },
                        CliCommand::Quit => {
                            info!("Bye");
                            break;
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
            Event::ConsoleError(err) => {
                error!("Console error: {}", err);
                break;
            }
            Event::ConsoleEmptyLine => {
                info!("Console empty line");
            }
        }
    }
    Ok(())
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
        .get(format!("{}/{}/{}", cfg.http_addr, "users", uid))
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
            debug!("Empty line");
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
