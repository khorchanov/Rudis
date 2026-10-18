use std::{ops::Index, str::Split};

use anyhow::Ok;

#[derive(Debug)]
pub enum Command {
    Ping,
    Put(String, String),
    Get(String),
    Del(String),
}

impl Command {
    pub fn try_parse(buffer: &String) -> anyhow::Result<Self> {
        let mut iterator = buffer.trim().split(" ");
        let command = match iterator.next() {
            Some(command) => command,
            None => return Err(anyhow::anyhow!("Unable to understand such command")),
        };

        match command {
            "PING" => Ok(Command::Ping),
            "GET" => Ok(Command::Get(get_next(&mut iterator)?)),
            "DEL" => Ok(Command::Del(get_next(&mut iterator)?)),
            "PUT" => Ok(Command::Put(
                get_next(&mut iterator)?,
                iterator.collect::<Vec<_>>().join(" "),
            )),
            _ => Err(anyhow::anyhow!("Unable to understand such command")),
        }
    }
}

fn get_next(iterator: &mut Split<'_, &str>) -> anyhow::Result<String> {
    iterator
        .next()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to understand such command"))
}
