use anyhow::anyhow;

use super::Command;
use crate::protocol::Literal;

pub(super) fn parse(mut args: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let key = args
        .next()
        .ok_or_else(|| anyhow!("Protocol Error"))?
        .into_string()?;
    Ok(Command::Get(key))
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::command::Command;

    #[test]
    fn resp_parse_get() {
        let input = b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n";
        let result = dbg!(Command::try_parse_resp(input).unwrap());
        assert_matches!(result, Some((Command::Get(key), bytes)) if key == "name" && bytes == input.len());
    }
}
