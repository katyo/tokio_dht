use std::rc::Rc;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::io::Error;

use futures::Future;
use futures::future::{ok};

use tokio_core::reactor::Handle;
//use tokio_service::Service;

use super::super::proto::{KTransError, KService};
use super::super::route::{Table};

use super::rpc::{BtDhtId, BtDhtQuery, BtDhtArg, BtDhtRes};
use super::{BtDhtHandler};

pub struct BtDhtService {
    table: Rc<RefCell<Table<BtDhtId>>>,
    service: KService<BtDhtQuery, BtDhtArg, BtDhtRes, BtDhtHandler>,
}

impl<'s> BtDhtService {
    pub fn new(node_id: BtDhtId, addr: &SocketAddr, handle: &Handle) -> (Self, Box<Future<Item = (), Error = Error> + 's>) {
        let table = Rc::new(RefCell::new(Table::new(node_id)));
        let handler = BtDhtHandler::new(table.clone());
        let (service, thread) = KService::new(handler, addr, handle);
        (BtDhtService {table, service}, thread)
    }

    pub fn ping_node(&self, addr: SocketAddr) -> Box<Future<Item = BtDhtId, Error = KTransError> + 's> {
        let table = self.table.borrow();
        let id = *table.node_id();
        Box::new(
            self.service.query(addr, BtDhtArg::Ping {id})
                .and_then(|res| {
                    match res {
                        BtDhtRes::Pong {id} => ok(id),
                        _ => unreachable!(), // TODO!!!
                    }
                })
        )
    }
}
