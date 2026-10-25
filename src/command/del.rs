use super::Command;
use crate::protocol::Literal;

pub(super) fn parse(args: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let keys = args
        .map(Literal::into_string)
        .collect::<anyhow::Result<Vec<String>>>()?;
    Ok(Command::Del(keys))
}
