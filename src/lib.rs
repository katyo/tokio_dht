#![feature(test)]
extern crate test;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate serde_bencode;
extern crate rand;
extern crate crypto_hashes;

#[macro_use]
extern crate log;

extern crate hexdump;

extern crate futures;
extern crate tokio_core;
extern crate tokio_service;

pub mod proto;
pub mod route;
//pub mod service;

pub mod id;
pub mod bt;

//pub use bt::dht::BtDhtHash;
//pub use service::BtDhtService;
