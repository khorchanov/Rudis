use anyhow::{Ok, anyhow, bail};

use crate::protocol::{self, Literal};

macro_rules! parse_argument {
    ($value:expr, [$($enum:ident :: $variant:ident),+ $(,)?]) => {
        match $value {
            $(
                Literal::String(key) | Literal::BulkString(key)
                    if key == stringify!($variant) => Ok($enum::$variant),
            )+
            Literal::String(key) | Literal::BulkString(key) => {
                bail!("{key} is not a valid argument for SET command")
            }
            _ => bail!("Wrong ARGUMENT format For SET command"),
        }
    };
}

#[derive(Debug)]
//TODO: add missing commands
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(String, String, Vec<SetArgument>),
    Get(String),
    Del(String), //TODO: can have many arguments
}

#[derive(Debug, PartialEq)]
pub enum SetArgument {
    NX,
    XX,
    GET,
}

impl TryFrom<Literal> for SetArgument {
    type Error = anyhow::Error;

    fn try_from(value: Literal) -> Result<Self, Self::Error> {
        parse_argument!(value, [SetArgument::NX, SetArgument::XX, SetArgument::GET])
    }
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
                    "PING" => {
                        let args = iterator.next().map(|lit| lit.into_string()).transpose()?;
                        Command::Ping(args)
                    }
                    "ECHO" => {
                        let message = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        Command::Echo(message)
                    }
                    "GET" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        Command::Get(key)
                    }
                    "DEL" => {
                        let key = iterator
                            .next()
                            .ok_or_else(|| anyhow!("Protocol Error"))?
                            .into_string()?;
                        Command::Del(key)
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
                        let args: Vec<SetArgument> = iterator
                            .map(SetArgument::try_from)
                            .collect::<anyhow::Result<Vec<SetArgument>>>(
                        )?;
                        Command::Set(key, value, args) //TODO: support putting an integer
                    }
                    _ => bail!("Unknown command"), //TODO: error should be returned properly
                },
                _ => bail!("Protocl error: Expected a first BulkString with the command"),
            };
            return Ok(Some((result, bytes_processed)));
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
        let result = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Ping(None), bytes)) if bytes == input.len());
    }

    #[test]
    fn resp_parse_ping_echo() {
        let input = b"*2\r\n$4\r\nPING\r\n$3\r\nmsg\r\n";
        let result = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Ping(Some(msg)), bytes)) if msg == "msg" && bytes == input.len());
    }

    #[test]
    fn resp_parse_get() {
        let input = b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n";
        let result = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Get(key), bytes)) if key == "name" && bytes == input.len());
    }

    #[test]
    fn resp_parse_put() {
        let input = b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$3\r\nbob\r\n";
        let result = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Set(key, value, _), bytes)) if key == "name" && value =="bob" && bytes == input.len());
    }
}
