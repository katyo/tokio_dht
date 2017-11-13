use std::net::SocketAddr;
use serde_bytes;

use super::serde::serde_socket_addr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KAddress (
    #[serde(with = "serde_socket_addr")]
    pub SocketAddr,
);

pub type KTransId = Vec<u8>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "y")]
pub enum KMessage<Query, Arg, Res> {
    #[serde(rename = "q")]
    Query {
        #[serde(rename = "t")]
        #[serde(with = "serde_bytes")]
        tid: KTransId,
        #[serde(rename = "q")]
        query: Query,
        #[serde(rename = "a")]
        arg: Arg,
    },
    #[serde(rename = "r")]
    Response {
        ip: Option<KAddress>,
        #[serde(rename = "t")]
        #[serde(with = "serde_bytes")]
        tid: KTransId,
        #[serde(rename = "r")]
        res: Res,
    },
    #[serde(rename = "e")]
    Error {
        ip: Option<KAddress>,
        #[serde(rename = "t")]
        #[serde(with = "serde_bytes")]
        tid: KTransId,
        #[serde(rename = "e")]
        error: (KError, String),
    },
}

serde_numeric_enum!(KError {
    Generic = 201,
    Server = 202,
    Protocol = 203,
    Method = 204,
});
