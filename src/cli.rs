use std::{any::TypeId, str::SplitAsciiWhitespace, unimplemented};

use crate::imports::*;
use crate::symbols::*;

pub fn parse(cmd: &str, state: ClientState) -> Result<CliCommand, CliParseError> {
    if cmd.is_ascii() {
        let mut tks = cmd.split_ascii_whitespace();
        if let Some(first) = tks.nth(0) {
            // is this even a command?
            if let Some('/') = first.chars().nth(0) {
                let rem_toks = &mut tks;
                // this is a command
                match first {
                    "/c" | "/g" => parse_c(rem_toks),
                    "/d" | "/u" => parse_u(rem_toks),
                    "/r" => parse_r(rem_toks),
                    "/s" => parse_s(rem_toks),
                    "/q" => parse_q(rem_toks),
                    "/j" => parse_j(rem_toks),
                    _ => Err(CliParseError::UnrecognizedCommand(first.to_owned()))
                }
            } else {
                // not a command
                if state == ClientState::Disconnected {
                    Err(CliParseError::CannotChatNow(cmd.to_owned()))
                } else {
                    Ok(CliCommand::Text(cmd.to_owned()))
                }
            }
        } else {
            Err(CliParseError::Empty)
        }
    } else {
        Err(CliParseError::NotAscii)
    }
}

pub fn parse_c(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    if let Some(chan) = rem_toks.nth(0) {
        if let Ok(gid) = chan.parse::<u32>() {
            Ok(CliCommand::SelectGroup(GroupId::from(gid)))
        } else {
            Err(CliParseError::TypeError(TypeId::of::<u32>()))
        }
    } else {
        Err(CliParseError::MissingExpected("gid"))
    }
}

pub fn parse_u(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    if let Some(chan) = rem_toks.nth(0) {
        if let Ok(uid) = chan.parse::<u32>() {
            Ok(CliCommand::SelectUser(UserId::from(uid)))
        } else {
            Err(CliParseError::TypeError(TypeId::of::<u32>()))
        }
    } else {
        Err(CliParseError::MissingExpected("uid"))
    }
}

// todo
pub fn parse_r(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    Err(CliParseError::NotImpl)
}

// todo
pub fn parse_s(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    Err(CliParseError::NotImpl)
}

// todo
pub fn parse_q(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    Err(CliParseError::NotImpl)
}

// todo
pub fn parse_j(rem_toks: &mut SplitAsciiWhitespace) -> Result<CliCommand, CliParseError> {
    Err(CliParseError::NotImpl)
}

pub enum CliParseError {
    CannotChatNow(String),
    Empty,
    NotAscii,
    UnrecognizedCommand(String),
    TypeError(TypeId),
    MissingExpected(&'static str),
    NotImpl
}

/**
# Commands

Commands may not be chained together.

`//` to escape the slash.

## Disconnected
```text
/j ::= {ip} |                   join a server
       {ip} ws:{port} |
       {ip} web:{port} |
       {ip} web:{port} ws:{port} |
       {ip} ws:{port} web:{port}
/r ::= {k}                      read an attribute
/s ::= {k}:{v | "v": string}    set an attribute (when v: string, quotes needed)
```

## Connected
```text
/c ::= {channel: string}        switch to channel {channel}
/g ::= {channel: string}        switch to channel {channel}
/d ::= {user: string}           switch to dm {user}
/u ::= {user: string}           switch to dm {user}
/r ::= {k: string}              read an attribute
/s ::= {k: string}:{v}          set an attribute
/q ::= {query}                  query
/{..}                           unrecognized command, will not be sent
{text}                          send {text} to currently active destination
```

# Types

Strings do not require surrounding codes.

```text
bool, int, uint, float, string
```
*/
pub enum CliCommand {
    GetAttr(String),
    SetAttr(String, CliType),
    Query(String),
    Text(String),
    Join {
        addr: String,
        ws_port: Option<u16>,
        web_port: Option<u16>
    },
    SelectGroup(GroupId),
    SelectUser(UserId)
}

pub enum CliType {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String)
}

#[derive(Eq, PartialEq, Clone)]
pub enum ClientState {
    Disconnected,
    Connected
}