use std::rc::Rc;
use std::cell::RefCell;

use futures::{Future};
use futures::future::{ok, err};

use tokio_service::Service;

use super::super::route::{Table};
use super::super::proto::{KError, KErrorKind};
use super::rpc::{BtDhtId, BtDhtArg, BtDhtRes};

pub struct BtDhtHandler {
    table: Rc<RefCell<Table<BtDhtId>>>,
}

impl BtDhtHandler {
    pub fn new(table: Rc<RefCell<Table<BtDhtId>>>) -> Self {
        BtDhtHandler { table }
    }
}

impl Service for BtDhtHandler {
    type Request = BtDhtArg;
    type Response = BtDhtRes;
    type Error = KError;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
    
    fn call(&self, arg: Self::Request) -> Self::Future {
        Box::new(match arg {
            BtDhtArg::Ping {id: _} => {
                let table = self.table.borrow();
                let id = *table.node_id();
                ok(BtDhtRes::Pong {id})
            },
            _ => {
                err(KError(KErrorKind::Method, "Method unimplemented".into()))
            },
        })
    }
}
