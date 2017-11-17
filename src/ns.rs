use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::str::FromStr;
use std::iter::IntoIterator;

use futures::Future;
use futures::future::{Either, ok, err, join_all};

use tokio_core::reactor::Handle;

use domain::bits::DNameBuf;
use domain::resolv::Resolver;
use domain::resolv::lookup::lookup_host;

pub fn resolve_hosts(addrs: Vec<String>, handle: &Handle) -> Box<Future<Item = Vec<SocketAddr>, Error = Error>> {
    let resolv = Resolver::new(handle);

    Box::new(join_all(addrs.into_iter().map(move |addr| {
        if let Ok(addr) = addr.parse() {
            Either::A(ok(addr))
        } else {
            let mut parts = addr.split(':');
            if let Some(host) = parts.next() {
                let port = if let Some(port) = parts.next() {
                    if let Ok(port) = port.parse() {
                        port
                    } else {
                        return Either::A(err(Error::new(ErrorKind::InvalidData, "Unable to parse port number")))
                    }
                } else {
                    6881
                };
                if let None = parts.next() {
                    let name = DNameBuf::from_str(host).unwrap();
                    let host_err = host.to_owned();
                    return Either::B(lookup_host(resolv.clone(), name)
                                     .map_err(move |_err| {
                                         Error::new(ErrorKind::Other, format!("Unable to resolve hostname: {}", host_err))
                                     })
                                     .and_then(move |addrs| {
                                         if let Some(addr) = addrs.iter().next() {
                                             ok(SocketAddr::new(addr, port))
                                         } else {
                                             err(Error::new(ErrorKind::NotFound, "Unknown hostname"))
                                         }
                                     }))
                }
            }
            Either::A(err(Error::new(ErrorKind::InvalidData, "Unable to parse address")))
        }
    })))
}
