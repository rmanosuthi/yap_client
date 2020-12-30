use std::unimplemented;

use crate::imports::*;
use crate::symbols::*;

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
pub fn parse_disconnected(cmd: &str, state: ClientState) -> Result<CliCommandDisconnected, String> {
    unimplemented!()
}

pub fn parse_connected(cmd: &str, state: ClientState) -> Result<CliCommandConnected, String> {
    unimplemented!()
}

pub enum CliCommandDisconnected {
    Join {
        addr: String,
        ws_port: Option<u16>,
        web_port: Option<u16>
    },
    GetAttr(String),
    SetAttr(String, CliType)
}

pub enum CliCommandConnected {
    SwitchToChannel(String),
    SwitchToDm(String),
    GetAttr(String),
    SetAttr(String, CliType),
    Query(String),
    Text(String)
}

pub enum CliType {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String)
}

pub enum ClientState {
    Disconnected,
    Connected
}