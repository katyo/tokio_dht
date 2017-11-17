use std::net::SocketAddr;
use std::collections::HashMap;

use super::{KTransId, KId};

type TransId = u16;
type TransKey = (SocketAddr, TransId);
type TransPool<Data> = HashMap<TransKey, Data>;

pub struct KTrans<Data> {
    last_tid: TransId,
    pool: TransPool<Data>,
}

impl<Data> KTrans<Data> {
    pub fn new() -> Self {
        KTrans {last_tid: 0, pool: HashMap::new()}
    }

    pub fn active(&self) -> usize {
        self.pool.len()
    }

    pub fn start(&mut self, addr: SocketAddr, data: Data) -> KId {
        self.last_tid += 1;
        let tid = self.last_tid;
        self.pool.insert((addr, tid), data);
        KId(addr, Some(KTransId(vec![(tid >> 8) as u8, tid as u8])))
    }

    pub fn end(&mut self, trans: &KId) -> Option<Data> {
        if let &KId(addr, Some(KTransId(ref tid))) = trans {
            if tid.len() == 2 {
                let tid = ((tid[0] as u16) << 8) | (tid[1] as u16);
                return self.pool.remove(&(addr, tid))
            }
        }
        None
    }
}
