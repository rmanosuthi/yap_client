mod imports {
    pub use crate::imports::*;
    pub use crate::symbols::*;
}
pub use self::shared::*;

mod shared {
    use super::imports::*;

    use crate::imports::*;
    use crate::symbols::*;
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct UserRecord {
        id: UserId,
        email: Option<String>,
        pubkey: Pubkey,
        hashed_pass: HashedPassword,
        alias: Option<String>,
        friends: Option<Vec<UserId>>,
        groups: Option<Vec<GroupId>>,
        motd: Option<String>,
        status: UserStatus,
        visibility: UserVisibility
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct GroupRecord {
        gid: GroupId,
        motd: Option<String>
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum UserStatus {
        Online,
        Offline,
        Invisible
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
        Public
    }
    
    impl Default for UserVisibility {
        fn default() -> Self {
            Self::FriendsOnly
        }
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
    pub struct UserId(u32);
    
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
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Pubkey(String);
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct HashedPassword(String);

    #[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
    pub struct LoginToken {
        pub tk: String,
    }

    impl LoginToken {
        pub fn new() -> LoginToken {
            LoginToken {
                tk: rand::thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(40)
                    .map(char::from)
                    .collect(),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
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
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ClientMessage(String);

    impl Display for ClientMessage {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<String> for ClientMessage {
        fn from(s: String) -> Self {
            ClientMessage(s)
        }
    }
    #[derive(Serialize, Deserialize)]
    pub struct ServerMessage {
        pub t: DateTime<Utc>,
        pub s: UserId,
        pub r: MessageActor,
        pub m: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum WsServerbound {
        NewUserMessage { to: UserId, c: ClientMessage },
        NewGroupMessage { to: GroupId, c: ClientMessage },
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum WsClientbound {
        NewUserMessage {
            from: UserId,
            c: ClientMessage,
            umid: UserMessageId,
        },
        NewGroupMessage {
            from: GroupId,
            c: ClientMessage,
            gmid: GroupMessageId,
        },
        FailedToSendMessage,
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
}
