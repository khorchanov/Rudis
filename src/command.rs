use anyhow::bail;

use crate::protocol::{self, Literal};

mod del;
mod echo;
mod exists;
mod get;
mod ping;
mod set;

pub use set::{SetArgumentExpiryRule, SetArgumentRule, SetCommand};

#[derive(Debug)]
//TODO: add missing commands
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(SetCommand),
    Get(String),
    Del(Vec<String>),
    Exists(Vec<String>),
}

impl Command {
    pub fn try_parse_resp(buffer: &[u8]) -> anyhow::Result<Option<(Self, usize)>> {
        let Some((literal, bytes_processed)) = protocol::try_parse_resp(buffer)? else {
            return Ok(None);
        };
        if let Literal::Array(elements) = literal {
            let mut iterator = elements.into_iter();
            let result = match iterator.next() {
                Some(Literal::BulkString(command)) => match command.as_str() {
                    "PING" => ping::parse(iterator)?,
                    "ECHO" => echo::parse(iterator)?,
                    "GET" => get::parse(iterator)?,
                    "DEL" => del::parse(iterator)?,
                    "EXISTS" => exists::parse(iterator)?,
                    "SET" => set::parse(iterator)?,
                    _ => bail!("Unknown command"), //TODO: error should be returned properly
                },
                _ => bail!("Protocl error: Expected a first BulkString with the command"),
            };
            return Ok(Some((result, bytes_processed)));
        }
        bail!("Protocl error, Request should be in a form of an array")
    }
}
