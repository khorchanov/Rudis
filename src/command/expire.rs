use std::time::Duration;

use crate::{command::Command, protocol::Literal};

pub(super) fn parse(
    mut args: impl Iterator<Item = Literal>,
    duration_interpreter_fn: impl FnOnce(u64) -> Duration,
) -> anyhow::Result<Command> {
    let key: String = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .into_anything()?;
    let ms = args
        .next()
        .map(Literal::into_anything::<u64>)
        .transpose()?
        .map(duration_interpreter_fn);
    Ok(Command::Expire(key, ms))
}

#[cfg(test)]
mod test {
    use std::{assert_matches, time::Duration};

    use crate::command::Command;

    #[test]
    fn resp_parse_expire_uses_seconds() {
        let input = b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$2\r\n10\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::Expire(key, Some(d)), bytes))
                if key == "k" && d == Duration::from_secs(10) && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_pexpire_uses_milliseconds() {
        let input = b"*3\r\n$7\r\nPEXPIRE\r\n$1\r\nk\r\n$4\r\n1500\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::Expire(key, Some(d)), bytes))
                if key == "k" && d == Duration::from_millis(1500) && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_expire_without_key_is_an_error() {
        let input = b"*1\r\n$6\r\nEXPIRE\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }

    #[test]
    fn resp_parse_expire_with_non_numeric_ttl_is_an_error() {
        let input = b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\nabc\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }

    #[test]
    fn resp_parse_expire_with_negative_ttl_is_an_error() {
        let input = b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$2\r\n-1\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }
}
