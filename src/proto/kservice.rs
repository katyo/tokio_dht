use std::fmt::Debug;
use std::marker::PhantomData;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use serde::ser::Serialize;
use serde::de::DeserializeOwned;

use futures::{Future, Sink, Stream};
use futures::future::{Either, Loop, loop_fn, ok, err};
use futures::sync::{oneshot, mpsc};

use tokio_core::reactor::Handle;
use tokio_core::net::UdpSocket;
use tokio_service::Service;

use super::{KError, KQueryArg, KCodec, KItem, KData, KTrans};

#[derive(Debug)]
pub enum KTransError {
    KError(KError),
    IOError(Error),
}

type KTransResponder<Res> = oneshot::Sender<Result<Res, KTransError>>;
struct KTransQuery<Arg, Res>(SocketAddr, Arg, KTransResponder<Res>);

pub struct KService<Query, Arg, Res, Handler> {
    query_tx: mpsc::Sender<KTransQuery<Arg, Res>>,
    phantom: PhantomData<(Query, Handler)>,
}

impl<'s, Query, Arg, Res, Handler> KService<Query, Arg, Res, Handler>
    where Query: 's + Serialize + DeserializeOwned + Debug + Eq,
          Arg: 's + Serialize + DeserializeOwned + Debug + KQueryArg<Query = Query>,
          Res: 's + Serialize + DeserializeOwned + Debug,
          Handler: 's + Service<Request = Arg, Response = Res, Error = KError>,
{
    pub fn query(&self, addr: SocketAddr, arg: Arg) -> Box<Future<Item = Res, Error = KTransError> + 's> {
        let (res_tx, res_rx) = oneshot::channel();
        Box::new(
            self.query_tx.clone().send(KTransQuery(addr, arg, res_tx))
                .map_err(|_| KTransError::IOError(Error::new(ErrorKind::Other, "Recv error")))
                .and_then(|_| {
                    res_rx.map_err(|_| KTransError::IOError(Error::new(ErrorKind::Other, "Recv error")))
                        .and_then(|response| {
                            match response {
                                Ok(res) => ok(res),
                                Err(error) => err(error),
                            }
                        })
                })
        )
    }
    
    pub fn new(handler: Handler, addr: &SocketAddr, handle: &Handle) -> (Self, Box<Future<Item = (), Error = Error> + 's>) {
        let trans: KTrans<KTransResponder<Res>> = KTrans::new();
        let codec: KCodec<Query, Arg, Res> = KCodec::new();
        let socket = UdpSocket::bind(addr, handle).unwrap();
        info!("Listening on: {}", socket.local_addr().unwrap());
        let (net_tx, net_rx) = socket.framed(codec).split();
        let (query_tx, query_rx) = mpsc::channel(1);
        // Compose event stream
        let event_rx = net_rx.map(Either::A)
            .select(query_rx.map(Either::B)
                    .map_err(|_| Error::new(ErrorKind::Other, "Query error")))
            .into_future();
        (KService { query_tx, phantom: PhantomData },
         Box::new(
             loop_fn(
                 (event_rx, net_tx, trans, handler),
                 |(event_rx, net_tx, mut trans, handler)| {
                     event_rx.map_err(|(err, ..)| {
                         error!("recv err: {}", err);
                         err
                     }).and_then(|(item, event_stream)| {
                         if let Some(item) = item {
                             let event_rx = event_stream.into_future();
                             match item {
                                 Either::A(KItem(id, msg)) => {
                                     match msg {
                                         KData::Query(arg) => {
                                             return Either::B(Either::A(handler.call(arg).then(|result| {
                                                 let resp = match result {
                                                     Ok(res) => KData::Response(res),
                                                     Err(err) => KData::Error(err),
                                                 };
                                                 net_tx.send(KItem(id, resp))
                                                     .and_then(|net_tx| {
                                                         ok(Loop::Continue((event_rx, net_tx, trans, handler)))
                                                     })
                                             })));
                                         },
                                         KData::Response(res) => {
                                             if let Some(res_tx) = trans.end(&id) {
                                                 let _ = res_tx.send(Ok(res));
                                             }
                                         },
                                         KData::Error(err) => {
                                             warn!("DHT Error response: {:?}", err);
                                             if let Some(res_tx) = trans.end(&id) {
                                                 let _ = res_tx.send(Err(KTransError::KError(err)));
                                             }
                                         },
                                     }
                                 },
                                 Either::B(KTransQuery(addr, arg, res_tx)) => {
                                     let id = trans.start(addr, res_tx);
                                     return Either::B(Either::B(net_tx.send(KItem(id, KData::Query(arg)))
                                         .and_then(|net_tx| {
                                             ok(Loop::Continue((event_rx, net_tx, trans, handler)))
                                         })))
                                 },
                             }
                             Either::A(ok(Loop::Continue((event_rx, net_tx, trans, handler))))
                         } else {
                             Either::A(ok(Loop::Break(())))
                         }
                     })
                 }
             )
         ))
    }
}
