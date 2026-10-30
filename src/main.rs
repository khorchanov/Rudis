use tokio::net::TcpListener;

use anyhow::Result;
use rudis::{Executor, handle_connection};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let executor = Executor::default();

    println!("Server listening on port 8080");

    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut executor = executor.clone();
        tokio::spawn(async move {
            let (reader, writer) = stream.split();
            if let Err(e) = handle_connection(reader, writer, &mut executor).await {
                eprintln!("Connection error: {e}");
            }
        });
    }
}
