extern crate futures;
extern crate tokio_core;
extern crate tokio_dht;
extern crate pretty_env_logger;

use futures::Future;

use tokio_core::reactor::Core;

use tokio_dht::bt::{BtDhtId, BtDhtService};

fn main() {
    pretty_env_logger::init().unwrap();
    println!("start...");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr = "0.0.0.0:6881".parse().unwrap();

    let node_id = BtDhtId::new();

    let (service, server) = BtDhtService::new(node_id, &addr, &handle);

    let query = service.ping_node("82.221.103.244:6881".parse().unwrap());

    handle.spawn(query.then(|res| {
        println!("Pong: {:?}!!!", res);
        Ok(())
    }));

    core.run(server).unwrap();
}
