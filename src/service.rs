//use std::io::{Error, Result};
use std::time::Duration;
use std::net::SocketAddr;
use std::io::{Error, ErrorKind};

use futures::{Future, Sink, Stream, Canceled};
use futures::future::{Either, Loop, loop_fn, ok};
use futures::sync::{oneshot, mpsc};

use tokio_core::reactor::{Handle, Interval};
use tokio_core::net::UdpSocket;

use super::proto::krpc::KMessage;
use super::proto::kcodec::KCodec;
use super::proto::bt_dht::{BtDhtHash, BtDhtMessage, BtDhtQuery, BtDhtArg, BtDhtRes};

#[derive(Debug)]
enum Command {
    AddNode(SocketAddr),
    FindNode(BtDhtHash, oneshot::Sender<SocketAddr>),
    GetPeers(BtDhtHash, oneshot::Sender<Vec<SocketAddr>>),
    Finalize,
}

pub struct BtDhtService {
    cmd_tx: mpsc::Sender<Command>,
}

pub type CommandFuture<Result> = Box<Future<Item = Result, Error = ()>>;

impl BtDhtService {
    pub fn find_node(&self, node_id: &BtDhtHash) -> CommandFuture<SocketAddr> {
        let (tx, rx) = oneshot::channel();
        Box::new(self.cmd_tx.clone()
                 .send(Command::FindNode(*node_id, tx))
                 .map_err(|_| Canceled)
                 .and_then(|_| rx)
                 .map_err(|_| ()))
    }
    
    pub fn get_peers(&self, info_hash: &BtDhtHash) -> CommandFuture<Vec<SocketAddr>> {
        let (tx, rx) = oneshot::channel();
        Box::new(self.cmd_tx.clone()
                 .send(Command::GetPeers(*info_hash, tx))
                 .map_err(|_| Canceled)
                 .and_then(|_| rx)
                 .map_err(|_| ()))
    }

    pub fn finalize(&self) -> CommandFuture<()> {
        Box::new(self.cmd_tx.clone()
                 .send(Command::Finalize)
                 .map_err(|_| ())
                 .and_then(|_| ok(())))
    }
    
    pub fn new(node_id: BtDhtHash, addr: &SocketAddr, handle: &Handle) -> (Self, Box<Future<Item = (), Error = Error>>) {
        let codec: KCodec<BtDhtMessage> = KCodec::new();
        let socket = UdpSocket::bind(addr, handle).unwrap();
        let tim_rx = Interval::new(Duration::new(30, 0), handle).unwrap();
        info!("Listening on: {}", socket.local_addr().unwrap());
        let (net_tx, net_rx) = socket.framed(codec).split();
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        // Compose event stream
        let com_rx = net_rx.map(Either::A) // mix incoming network messages
            .select(cmd_rx.map(Either::B) // with user API commands
                    .map_err(|_| Error::new(ErrorKind::Other, "Command error")))
            .map(Either::A)
            .select(tim_rx.map(Either::B)) // and add periodic timer events
            .into_future(); // treat our events stream as future
        (BtDhtService { cmd_tx },
         Box::new(
             loop_fn(
                 (com_rx, net_tx),
                 move |(rx, net_tx)| {
                     rx.map_err(|(err, _rx)| {
                         error!("recv err: {}", err);
                         err
                     }).and_then(move |(item, rx)| {
                         if let Some(item) = item {
                             match item {
                                 Either::A(Either::A((from, msg))) => { // incoming network message
                                     //debug!("v net: {:?}", msg);
                                     match msg {
                                         KMessage::Query {tid, query, arg} => {
                                             //info!("v from: {}, tid: {:?}, query: {:?}, {:?}", from, tid, query, arg);
                                         },
                                         KMessage::Response {tid, res, ip} => {
                                             //info!("v from: {}, tid: {:?}, response: {:?}, ip: {:?}", from, tid, res, ip);
                                         },
                                         KMessage::Error {tid, error: (code, message), ..} => {
                                             //info!("v from: {}, tid: {:?}, error: {:?} {}", from, tid, code, message);
                                         },
                                     }
                                 },
                                 Either::A(Either::B(cmd)) => { // incoming service command
                                     match cmd {
                                         Command::Finalize => {
                                             return Either::A(Either::B(ok(Loop::Break(()))))
                                         },
                                         Command::FindNode(target, _res_tx) => {
                                             let id = node_id.clone();
                                             let q: BtDhtMessage = KMessage::Query {
                                                 tid: "aa".into(),
                                                 query: BtDhtQuery::FindNode,
                                                 arg: BtDhtArg::FindNode {id, target},
                                             };
                                             let peer_addr = "82.221.103.244:6881".parse().unwrap();
                                             return Either::B(net_tx.send((peer_addr, q))
                                                              //.map_err(|err| { error!("Send error: {:?}", err); err })
                                                              .and_then(|net_tx| {
                                                                  ok(Loop::Continue((
                                                                      rx.into_future(),
                                                                      net_tx,
                                                                  )))
                                                              }))
                                         },
                                         _ => (),
                                     };
                                     println!("v cmd: {:?}", cmd);
                                 },
                                 Either::B(_) => { // refresh interval reached
                                     info!("refresh routing table");
                                 },
                             };
                             Either::A(Either::A(ok(Loop::Continue((
                                 rx.into_future(),
                                 net_tx,
                             )))))
                         } else {
                             Either::A(Either::B(ok(Loop::Break(()))))
                         }
                     })
                 })))
    }
}
