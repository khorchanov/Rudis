use anyhow::Result;
use rudis::{
    executor::{Executor, SystemClock},
    handle_request,
    store::HashMapStore,
};
use std::net::TcpListener;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    let mut executor = Executor {
        store: HashMapStore::new(),
        clock: SystemClock {},
    };

    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_request(stream.try_clone()?, stream, &mut executor)?, //TODO: this is ugly
            Err(err) => eprintln!("Connection failed {err}"),
        }
    }
    Ok(())
}
