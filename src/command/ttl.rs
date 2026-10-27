use crate::{command::Command, protocol::Literal};

#[derive(Debug)]
pub enum TtlReturnUnit {
    Seconds,
    Millis,
}

pub(super) fn parse(
    mut args: impl Iterator<Item = Literal>,
    unit: TtlReturnUnit,
) -> anyhow::Result<Command> {
    let key: String = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .into_anything()?;
    Ok(Command::Ttl(key, unit))
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::command::{Command, TtlReturnUnit};

    #[test]
    fn resp_parse_ttl_uses_seconds() {
        let input = b"*2\r\n$3\r\nTTL\r\n$1\r\nk\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::Ttl(key, TtlReturnUnit::Seconds), bytes))
                if key == "k" && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_pttl_uses_milliseconds() {
        let input = b"*2\r\n$4\r\nPTTL\r\n$1\r\nk\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::Ttl(key, TtlReturnUnit::Millis), bytes))
                if key == "k" && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_ttl_without_key_is_an_error() {
        let input = b"*1\r\n$3\r\nTTL\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }

    #[test]
    fn resp_parse_pttl_without_key_is_an_error() {
        let input = b"*1\r\n$4\r\nPTTL\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }
}
