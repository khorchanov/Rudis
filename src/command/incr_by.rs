use crate::{command::Command, protocol::Literal};

pub(super) fn parse(mut args: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let key = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .into_string()?;
    let by = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .into_i64()?;
    Ok(Command::IncrBy(key, by))
}
