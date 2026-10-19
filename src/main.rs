use tokio::net::TcpListener;

use anyhow::Result;
use rudis::{Executor, HashMapStore, SystemClock, handle_connection};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let mut executor = Executor {
        store: HashMapStore::default(),
        clock: SystemClock {},
    };

    println!("Server listening on port 8080");

    loop {
        let (mut stream, _) = listener.accept().await?;
        let (reader, writer) = stream.split();
        handle_connection(reader, writer, &mut executor).await?;
    }
}
