use anyhow::anyhow;

use super::Command;
use crate::{protocol::Literal, utils::now_ms};

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

pub(super) fn parse(mut iterator: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let key = iterator
        .next()
        .ok_or_else(|| anyhow!("Protocol Error"))?
        .into_anything()?;
    let value = iterator
        .next()
        .ok_or_else(|| anyhow!("Protocol Error"))?
        .into_anything()?;
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
                let duration: u64 = iterator
                    .next()
                    .ok_or_else(|| anyhow!("Protocol Error"))?
                    .into_anything()?;
                let expires_after = if arg == "EX" {
                    now_ms() + duration * 1000
                } else {
                    now_ms() + duration
                };
                expiry_rule = Some(SetArgumentExpiryRule::ExpiresAfterMs(expires_after))
            }
            arg @ ("EXAT" | "PXAT") => {
                let timestamp = iterator
                    .next()
                    .ok_or_else(|| anyhow!("Protocol Error"))?
                    .into_anything()?;
                let expires_after = if arg == "EX" {
                    timestamp * 1000
                } else {
                    timestamp
                };
                expiry_rule = Some(SetArgumentExpiryRule::ExpiresAtMs(expires_after))
            }
            _ => {}
        }
    }
    Ok(Command::Set(SetCommand {
        key,
        value,
        return_old_value,
        set_rule,
        expiry_rule,
    })) //TODO: support putting an integer
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::command::{Command, SetCommand};

    #[test]
    fn resp_parse_put() {
        let input = b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$3\r\nbob\r\n";
        let result: Option<(Command, usize)> = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Set(SetCommand{key,value, return_old_value:_, set_rule:_, expiry_rule:_ }), bytes)) if key == "name" && value =="bob" && bytes == input.len());
    }
}
