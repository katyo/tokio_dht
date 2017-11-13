use std::net::SocketAddr;
use std::io::{Error, ErrorKind, Result};
use std::fmt::Debug;
use std::marker::PhantomData;

use serde::ser::Serialize;
use serde::de::DeserializeOwned;

use hexdump::hexdump_iter;

use serde_bencode::ser::to_bytes;
use serde_bencode::de::from_bytes;

use tokio_core::net::{UdpCodec};

pub struct KCodec<Msg> {
    phantom: PhantomData<Msg>,
}

impl<Msg> KCodec<Msg>
    where Msg: Serialize + DeserializeOwned
{
    pub fn new() -> Self {
        KCodec {
            phantom: PhantomData,
        }
    }
}

impl<Msg> UdpCodec for KCodec<Msg>
    where Msg: Serialize + DeserializeOwned + Debug,
{
    type In = (SocketAddr, Msg);
    type Out = (SocketAddr, Msg);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        trace!("recv from: {}, packet:", addr);
        for line in hexdump_iter(buf) {
            trace!("    {}", line);
        }
        let msg = from_bytes(buf)
            .map_err(|err| Error::new(ErrorKind::InvalidData,
                                      format!("Decode error: {}", err)))?;
        debug!("recv from: {}, message: {:?}", addr, msg);
        Ok((*addr, msg))
    }

    fn encode(&mut self, (addr, msg): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        debug!("send to: {}, message: {:?}", addr, msg);
        let buf = to_bytes(&msg).unwrap();
        trace!("send to: {}, packet:", addr);
        for line in hexdump_iter(&buf) {
            trace!("    {}", line);
        }
        into.extend(buf);
        addr
    }
}
