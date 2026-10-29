use std::time::Duration;

use anyhow::bail;

use crate::protocol::{self, Literal};

mod parser;
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
    Incr(String),
    IncrBy(String, i64),
    Expire(String, Option<Duration>),
    Ttl(String, TtlReturnUnit),
    Persist(String),
}

#[derive(Debug)]
pub enum TtlReturnUnit {
    Seconds,
    Millis,
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
                    "PING" => Command::Ping(parser::parse_opt(&mut iterator)?),
                    "ECHO" => Command::Echo(parser::parse(&mut iterator)?),
                    "GET" => Command::Get(parser::parse(&mut iterator)?),
                    "PERSIST" => Command::Persist(parser::parse(&mut iterator)?),
                    "INCR" => Command::Incr(parser::parse(&mut iterator)?),
                    "INCRBY" => Command::IncrBy(
                        parser::parse(&mut iterator)?,
                        parser::parse(&mut iterator)?,
                    ),
                    "EXPIRE" => Command::Expire(
                        parser::parse(&mut iterator)?,
                        parser::parse_opt(&mut iterator)?.map(Duration::from_secs),
                    ),
                    "PEXPIRE" => Command::Expire(
                        parser::parse(&mut iterator)?,
                        parser::parse_opt(&mut iterator)?.map(Duration::from_millis),
                    ),
                    "TTL" => {
                        Command::Ttl(parser::parse(&mut iterator)?, TtlReturnUnit::Seconds)
                    }
                    "PTTL" => {
                        Command::Ttl(parser::parse(&mut iterator)?, TtlReturnUnit::Millis)
                    }
                    "DEL" => Command::Del(parser::parse_vec(&mut iterator)?),
                    "EXISTS" => Command::Exists(parser::parse_vec(&mut iterator)?),
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
