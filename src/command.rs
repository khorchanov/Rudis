use std::str::Split;

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
            "SET" => Ok(Command::Set(
                get_next(&mut iterator)?,
                iterator.collect::<Vec<_>>().join(" "),
            )),
            _ => Err(anyhow::anyhow!("Unable to understand such command")),
        }
    }

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
                            .as_string()?;
                        return Ok(Command::Get(key));
                    }
                    "DEL" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .as_string()?;
                        return Ok(Command::Del(key));
                    }
                    "SET" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .as_string()?;
                        let value = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .as_string()?;
                        return Ok(Command::Set(key, value)); //TODO: support putting an integer
                    }
                    _ => bail!("Unknown command"), //TODO: error should be returned properly
                },
                _ => bail!("Protocl error, malformed request"),
            }
        }
        bail!("Protocl error, malformed request")
    }
}

fn get_next(iterator: &mut Split<'_, &str>) -> anyhow::Result<String> {
    iterator
        .next()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to understand such command"))
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
