use std::net::SocketAddr;
use std::str::FromStr;
use std::iter::IntoIterator;

use futures::Future;
use futures::future::{Either, ok, join_all};

use tokio_core::reactor::Handle;

use domain::bits::DNameBuf;
use domain::resolv::Resolver;
use domain::resolv::lookup::lookup_host;

#[derive(Debug)]
pub enum ResolveError {
    InvalidAddr(String),
    InvalidPort(String),
    Unresolved(String),
}

pub type ResolveResult = Result<SocketAddr, ResolveError>;

pub fn resolve_hosts(addrs: Vec<String>, handle: &Handle) -> Box<Future<Item = Vec<ResolveResult>, Error = ()>> {
    let resolv = Resolver::new(handle);

    Box::new(join_all(addrs.into_iter().map(move |addr| {
        if let Ok(addr) = addr.parse() {
            Either::A(ok(Ok(addr)))
        } else {
            let addr_orig = addr.clone();
            let mut parts = addr.split(':');
            if let Some(host) = parts.next() {
                let port = if let Some(port) = parts.next() {
                    if let Ok(port) = port.parse() {
                        port
                    } else {
                        return Either::A(ok(Err(ResolveError::InvalidPort(port.to_owned()))))
                    }
                } else {
                    6881
                };
                if let None = parts.next() {
                    let name = DNameBuf::from_str(host).unwrap();
                    let host_err = host.to_owned();
                    return Either::B(lookup_host(resolv.clone(), name)
                                     .then(move |result| {
                                         match result {
                                             Ok(addrs) => {
                                                 if let Some(addr) = addrs.iter().next() {
                                                     ok(Ok(SocketAddr::new(addr, port)))
                                                 } else {
                                                     ok(Err(ResolveError::Unresolved(host_err)))
                                                 }
                                             },
                                             Err(_) => {
                                                 ok(Err(ResolveError::Unresolved(host_err)))
                                             },
                                         }
                                     })
                    )
                }
            }
            Either::A(ok(Err(ResolveError::InvalidAddr(addr_orig))))
        }
    })))
}
