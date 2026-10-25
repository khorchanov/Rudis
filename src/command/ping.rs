use super::Command;
use crate::protocol::Literal;

pub(super) fn parse(mut args: impl Iterator<Item = Literal>) -> anyhow::Result<Command> {
    let message = args.next().map(Literal::into_string).transpose()?;
    Ok(Command::Ping(message))
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::command::Command;

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
}
