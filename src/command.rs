use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Ok, anyhow, bail};

use crate::protocol::{self, Literal};

#[derive(Debug)]
//TODO: add missing commands
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(SetCommand),
    Get(String),
    Del(String), //TODO: can have many arguments
}

#[derive(Debug)]
pub struct SetCommand {
    pub key: String,
    pub value: String,
    pub return_old_value: bool,
    pub set_rule: Option<SetArgumentRule>,
    pub expiry_rule: Option<SetArgumentExpiryRule>,
}

#[derive(Debug, PartialEq)]
pub enum SetArgumentRule {
    NX,
    XX,
}

#[derive(Debug, PartialEq)]
pub enum SetArgumentExpiryRule {
    ExpiresAfterMs(u64),
    ExpiresAtMs(u64),
    KeepTtl,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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
                        let mut set_rule = None;
                        let mut expiry_rule = None;
                        let mut return_old_value = false;
                        while let Some(Literal::BulkString(next)) = iterator.next() {
                            //TODO: beware of hidden errors
                            match next.as_str() {
                                "NX" => set_rule = Some(SetArgumentRule::NX),
                                "XX" => set_rule = Some(SetArgumentRule::XX),
                                "KEEPTTL" => expiry_rule = Some(SetArgumentExpiryRule::KeepTtl),
                                "GET" => return_old_value = true,
                                arg @ ("EX" | "PX") => {
                                    let duration = iterator
                                        .next()
                                        .ok_or_else(|| anyhow!("Protocol Error"))?
                                        .into_u64()?;
                                    let expires_after = if arg == "EX" {
                                        now_ms() + duration * 1000
                                    } else {
                                        now_ms() + duration
                                    };
                                    expiry_rule =
                                        Some(SetArgumentExpiryRule::ExpiresAfterMs(expires_after))
                                }
                                arg @ ("EXAT" | "PXAT") => {
                                    let timestamp = iterator
                                        .next()
                                        .ok_or_else(|| anyhow!("Protocol Error"))?
                                        .into_u64()?;
                                    let expires_after = if arg == "EX" {
                                        timestamp * 1000
                                    } else {
                                        timestamp
                                    };
                                    expiry_rule =
                                        Some(SetArgumentExpiryRule::ExpiresAfterMs(expires_after))
                                }
                                _ => {}
                            }
                        }
                        let command = SetCommand {
                            key,
                            value,
                            return_old_value,
                            set_rule,
                            expiry_rule,
                        };
                        Command::Set(command) //TODO: support putting an integer
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
    use super::SetCommand;

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
        let result: Option<(Command, usize)> = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Set(SetCommand{key,value, return_old_value:_, set_rule:_, expiry_rule:_ }), bytes)) if key == "name" && value =="bob" && bytes == input.len());
    }
}
