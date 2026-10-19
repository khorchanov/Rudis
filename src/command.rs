use anyhow::{Ok, anyhow, bail};

use crate::protocol::{self, Literal};

#[derive(Debug)]
//TODO: add missing commands
pub enum Command {
    Ping, //TODO: can have an optional args
    Set(String, String),
    Get(String),
    Del(String), //TODO: can have many arguments
}

impl Command {
    pub fn try_parse_resp(buffer: &[u8]) -> anyhow::Result<Self> {
        let parsed = protocol::try_parse(buffer)?;
        if let Literal::Array(elements) = parsed {
            let mut iterator = elements.into_iter();
            match iterator.next() {
                Some(Literal::BulkString(command)) => match command.as_str() {
                    "PING" => return Ok(Command::Ping),
                    "GET" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        return Ok(Command::Get(key));
                    }
                    "DEL" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        return Ok(Command::Del(key));
                    }
                    "SET" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        let value = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        return Ok(Command::Set(key, value)); //TODO: support putting an integer
                    }
                    _ => bail!("Unknown command"), //TODO: error should be returned properly
                },
                _ => bail!("Protocl error: Expected a first BulkString with the command"),
            }
        }
        bail!("Protocl error, Request should be in a form of an array")
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::Command;

    #[test]
    fn resp_parse_ping() {
        let input = b"*1\r\n$4\r\nPING\r\n";
        let command = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(command, Command::Ping);
    }

    #[test]
    fn resp_parse_get() {
        let input = b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n";
        let command = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(command, Command::Get(key) if key == "name");
    }

    #[test]
    fn resp_parse_put() {
        let input = b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$3\r\nbob\r\n";
        let command = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(command, Command::Set(key, value) if key == "name" && value =="bob");
    }
}
