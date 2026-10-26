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

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::command::Command;

    #[test]
    fn resp_parse_incr_by() {
        let input = b"*3\r\n$6\r\nINCRBY\r\n$1\r\nk\r\n$1\r\n5\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::IncrBy(key, 5), bytes)) if key == "k" && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_incr_by_negative() {
        let input = b"*3\r\n$6\r\nINCRBY\r\n$1\r\nk\r\n$2\r\n-3\r\n";
        let result = Command::try_parse_resp(input).unwrap();
        assert_matches!(
            result,
            Some((Command::IncrBy(key, -3), bytes)) if key == "k" && bytes == input.len()
        );
    }

    #[test]
    fn resp_parse_incr_by_without_key_is_an_error() {
        let input = b"*1\r\n$6\r\nINCRBY\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }

    #[test]
    fn resp_parse_incr_by_without_amount_is_an_error() {
        let input = b"*2\r\n$6\r\nINCRBY\r\n$1\r\nk\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }

    #[test]
    fn resp_parse_incr_by_with_non_numeric_amount_is_an_error() {
        let input = b"*3\r\n$6\r\nINCRBY\r\n$1\r\nk\r\n$3\r\nabc\r\n";
        assert!(Command::try_parse_resp(input).is_err());
    }
}
