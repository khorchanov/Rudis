use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::Result;

use crate::command::Command;

mod command;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream)?,
            Err(err) => eprintln!("Connection failed {err}"),
        }
    }

    Ok(())
}

fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut buffer = String::new();
    let read_ops = reader.read_line(&mut buffer);
    match read_ops {
        Ok(0) => {}
        Err(e) => return Err(e.into()),
        Ok(_) => {
            let command = Command::try_parse(buffer)?;
            let processing_result = command.process()?;
            let mut writer = BufWriter::new(&stream);
            writer.write_all(processing_result.into_bytes().as_slice())?;
        }
    }
    Ok(())
}
