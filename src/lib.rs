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

pub mod proto;
pub mod service;

pub use proto::bt_dht::BtDhtHash;
pub use service::BtDhtService;
