use anyhow::anyhow;

use super::Command;
use crate::protocol::Literal;

pub(super) fn parse(mut args: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let key = args
        .next()
        .ok_or_else(|| anyhow!("Protocol Error"))?
        .into_string()?;
    Ok(Command::Incr(key))
}
