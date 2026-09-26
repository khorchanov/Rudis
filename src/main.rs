use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::Result;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream),
            Err(err) => eprintln!("Connection failed {err}"),
        }
    }

    Ok(())
}

fn handle_connection(stream: TcpStream) {
    let mut reader = BufReader::new(&stream);
    let mut buffer = String::new();
    let read_ops = reader.read_line(&mut buffer);
    match read_ops {
        Ok(0) => {},
        Err(e) => eprintln!("Cannot read buffer from connection {e}"),
        Ok(_) => {
            let mut writer = BufWriter::new(&stream);
            if let Err(e) = writer.write_all(&format!("OK {buffer}").into_bytes()) {
                eprintln!("Cannot write into buffer {e}");
                //TODO: handle writing issues
            }
        }
    }
}
