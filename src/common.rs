mod imports {
    pub use crate::imports::*;
    pub use crate::symbols::*;
}
pub use self::shared::*;

mod shared {
    use std::{convert::TryFrom, num::ParseIntError, str::FromStr};

    use super::imports::*;

    use crate::imports::*;
    use crate::symbols::*;

    /// How much information should be omitted when sending `UserRecord` to the client.
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum UserMaskLevel {
        /// Allow everything through, **including the hashed password!**
        SelfUse,
        /// Allow everything except the hashed password.
        HidePass,
        /// Allow everything except the hashed password and email.
        HidePassEmail,
        /// Allow everything except hashed password, email, friends, and groups.
        HidePassEmailMembership,
    }

    /// Public-facing version of `UserRecord`. Can be safely sent to the client.
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PublicUserRecord {
        pub uid: UserId,
        pub email: Option<String>,
        pub pubkey: Pubkey,
        pub hashed_pass: Option<HashedPassword>,
        pub alias: Option<String>,
        pub friends: Option<Vec<UserId>>,
        pub groups: Option<Vec<GroupId>>,
        pub motd: Option<String>,
        pub online: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PublicUserMessage {
        pub umid: UserMessageId,
        pub from: UserId,
        pub to: UserId,
        pub time_posted: DateTime<Utc>,
        pub content: ClientMessage,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct GroupRecord {
        pub gid: GroupId,
        pub motd: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum UserStatus {
        Online,
        Offline,
        Invisible,
    }

    impl Default for UserStatus {
        fn default() -> Self {
            Self::Offline
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum UserVisibility {
        Private,
        FriendsOnly,
        Public,
    }

    impl Default for UserVisibility {
        fn default() -> Self {
            Self::FriendsOnly
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy)]
    pub struct UserId(u32);

    impl FromStr for UserId {
        type Err = ParseIntError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let tmp = u32::from_str(s)?;
            Ok(Self(tmp))
        }
    }

    impl Display for UserId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<u32> for UserId {
        fn from(i: u32) -> Self {
            UserId(i)
        }
    }

    impl Into<u32> for UserId {
        fn into(self) -> u32 {
            self.0
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct GroupId(u32);

    impl Display for GroupId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<u32> for GroupId {
        fn from(i: u32) -> Self {
            GroupId(i)
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserMessageId(u64);

    impl Display for UserMessageId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<u64> for UserMessageId {
        fn from(i: u64) -> Self {
            UserMessageId(i)
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct GroupMessageId(u64);

    impl Display for GroupMessageId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<u64> for GroupMessageId {
        fn from(i: u64) -> Self {
            GroupMessageId(i)
        }
    }

    /// Public key for message signing. Server performs no validation!
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Pubkey(String);

    impl From<String> for Pubkey {
        fn from(s: String) -> Self {
            Pubkey(s)
        }
    }

    /// Hashed password. Server performs no validation!
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct HashedPassword(String);

    impl From<String> for HashedPassword {
        fn from(s: String) -> Self {
            HashedPassword(s)
        }
    }

    pub const LT_LEN: usize = 40;

    pub fn alphanumeric_len(value: &str, len: usize) -> bool {
        value
            .chars()
            .all(|c| char::is_alphanumeric(c) && char::is_ascii(&c) && !char::is_whitespace(c))
            && value.chars().count() == len
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
    pub struct LoginToken {
        pub tk: String,
    }

    impl LoginToken {
        pub fn new() -> LoginToken {
            LoginToken {
                tk: rand::thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(LT_LEN)
                    .map(char::from)
                    .collect(),
            }
        }
    }

    impl FromStr for LoginToken {
        type Err = usize;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if alphanumeric_len(s, LT_LEN) {
                Ok(LoginToken { tk: s.to_owned() })
            } else {
                Err(s.chars().count())
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum MessageActor {
        Dm(UserId),
        Group(GroupId),
    }

    #[derive(Serialize, Deserialize)]
    pub enum HistoryQuery {
        Unseen,
        Interval {
            from: DateTime<Utc>,
            to: DateTime<Utc>,
        },
        Since(DateTime<Utc>),
    }

    impl<T> From<T> for WsClientboundPayload
    where
        T: ClientboundPayload,
    {
        fn from(i: T) -> Self {
            i.make_payload()
        }
    }
    /** Message that will be sent over ws to the client.
    The server shouldn't interfere with this, nor should it be processed in any way.
    */
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum WsClientboundPayload {
        NewMessage(PublicUserMessage),
        NewMessages(Vec<PublicUserMessage>),
        MessageSent(UserMessageId),
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct RegisterRequest {
        pub email: String,
        pub password_hash: String,
        pub pubkey: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct LoginRequest {
        pub email: String,
        pub password_hash: String,
    }
    pub trait ClientboundPayload
    where
        Self: Sized,
    {
        fn make_payload(self) -> WsClientboundPayload;
    }

    impl ClientboundPayload for PublicUserMessage {
        fn make_payload(self) -> WsClientboundPayload {
            WsClientboundPayload::NewMessage(self)
        }
    }

    impl ClientboundPayload for Vec<PublicUserMessage> {
        fn make_payload(self) -> WsClientboundPayload {
            WsClientboundPayload::NewMessages(self)
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum WsServerboundPayload {
        NewUserMessage { to: UserId, content: ClientMessage },
    }

    impl Into<tungstenite::Message> for WsServerboundPayload {
        fn into(self) -> tungstenite::Message {
            tungstenite::Message::Text(serde_json::to_string(&self).unwrap())
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ClientMessage(String);

    impl From<String> for ClientMessage {
        fn from(s: String) -> Self {
            ClientMessage(s)
        }
    }

    impl Display for ClientMessage {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}
