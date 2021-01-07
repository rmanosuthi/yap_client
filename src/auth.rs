use std::convert::TryFrom;

use crate::imports::*;
use crate::symbols::*;
use openssl::{error::ErrorStack, pkey::{Public, Private}, rsa::{Padding, Rsa}};

#[derive(Serialize, Deserialize)]
pub struct LocalServerEntry {
    pub http_addr: String,
    pub ws_addr: String,
    pub email: String,
    pub phash: String,
    pub identity: LocalIdentity
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalIdentity {
    pub privkey: String,
    pub pubkey: String
}

pub struct InMemoryKey {
    rsa_priv: Rsa<Private>,
    rsa_pub: Rsa<Public>
}

impl InMemoryKey {
    /// Input message was signed using **private key of self.**
    pub fn encrypt(&self, msg: &str) -> Option<ClientMessage> {
        let mut res = vec![0; 256];
        let bytes_written = self.rsa_priv.private_encrypt(msg.to_string().as_bytes(), &mut res, Padding::PKCS1).ok()?;
        let content = hex::encode(res);
        Some(ClientMessage::from(content))
    }
}

impl TryFrom<LocalIdentity> for InMemoryKey {
    type Error = ErrorStack;

    fn try_from(value: LocalIdentity) -> Result<Self, Self::Error> {
        let rsa_priv = Rsa::private_key_from_pem(value.privkey.as_bytes())?;
        let rsa_pub = Rsa::public_key_from_pem(value.pubkey.as_bytes())?;
        Ok(Self {
            rsa_priv,
            rsa_pub
        })
    }
}

impl PublicUserRecord {
    /// Input message was signed with **private key of origin.**
    pub fn decrypt(&self, msg: ClientMessage) -> Option<String> {
        let in_bytes = msg.to_string();
        let pubkey = Rsa::public_key_from_pem(self.pubkey.to_string().as_bytes()).ok()?;
        let mut res = vec![0; 10000];
        let unhexed = hex::decode(in_bytes.as_bytes()).ok()?;
        debug!("unhex ok");
        let bytes_written = pubkey.public_decrypt(&unhexed, &mut res, Padding::PKCS1).ok()?;
        debug!("content decode ok");
        String::from_utf8(res).ok()
    }
}