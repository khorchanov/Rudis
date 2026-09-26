use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::Result;

use crate::{command::Command, store::{HashMapStore, Store}};

mod command;
mod store;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    let mut store = HashMapStore::new();

    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream, &mut store)?,
            Err(err) => eprintln!("Connection failed {err}"),
        }
    }

    Ok(())
}

fn handle_connection(stream: TcpStream, store : &mut impl Store) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut buffer = String::new();
    let read_ops = reader.read_line(&mut buffer);
    match read_ops {
        Ok(0) => {}
        Err(e) => return Err(e.into()),
        Ok(_) => {
            let command = Command::try_parse(buffer)?;
            let processing_result: Option<String> = command.process(store)?;
            let mut writer = BufWriter::new(&stream);
            writer.write_all(format!("OK {}", processing_result.unwrap_or_default()).into_bytes().as_slice())?;
        }
    }
    Ok(())
}
