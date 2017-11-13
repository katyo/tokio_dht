extern crate futures;
extern crate tokio_core;
extern crate tokio_dht;
extern crate pretty_env_logger;

use futures::Future;

use tokio_core::reactor::Core;

use tokio_dht::{BtDhtHash, BtDhtService};

fn main() {
    pretty_env_logger::init().unwrap();
    println!("start...");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr = "0.0.0.0:6881".parse().unwrap();

    let node_id = BtDhtHash::new();

    let (service, server) = BtDhtService::new(node_id, &addr, &handle);

    let query = service.find_node(&node_id);

    handle.spawn(query.then(|_| Ok(())));

    core.run(server).unwrap();
}
