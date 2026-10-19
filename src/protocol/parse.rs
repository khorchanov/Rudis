use anyhow::bail;

use super::Literal;

pub fn try_parse(buffer: &[u8]) -> anyhow::Result<Literal> {
    try_parse_resp(buffer)?.strip()
}

#[derive(Debug)]
struct ParseResult {
    literal: Literal,
    leftover: Option<Vec<u8>>,
}

impl ParseResult {
    fn strip(self) -> anyhow::Result<Literal> {
        if let Some(content) = self.leftover
            && !content.is_empty()
        {
            bail!("Unfinished Literal")
        }
        Ok(self.literal)
    }
}

fn try_parse_resp(buffer: &[u8]) -> anyhow::Result<ParseResult> {
    //TODO: support bytes input
    match buffer.first() {
        Some(b':') => {
            let end = find_crlf(buffer).unwrap();
            let content = parse_number(buffer.get(1..end))?;
            let leftover = buffer.get(end + 2..).map(|bytes| bytes.to_vec());
            Ok(ParseResult {
                literal: Literal::Integer(content),
                leftover,
            })
        }
        Some(b'+') => {
            let end = find_crlf(buffer).unwrap();
            let content = parse_string(buffer.get(1..end))?;
            let leftover = buffer.get(end + 2..).map(|bytes| bytes.to_vec());
            Ok(ParseResult {
                literal: Literal::String(content.to_owned()),
                leftover,
            })
        }
        Some(b'$') => {
            let size_end = find_crlf(buffer).unwrap();
            let content_size = parse_number(buffer.get(1..size_end))?;
            if content_size < 0 {
                return Ok(ParseResult {
                    literal: Literal::NullBulkString,
                    leftover: None,
                });
            }
            let content_start = size_end + 2;
            let content =
                parse_string(buffer.get(content_start..content_start + content_size as usize))?;
            let element_end = content_start + content_size as usize + 2;
            let leftover = buffer.get(element_end..).map(|bytes| bytes.to_vec());
            Ok(ParseResult {
                literal: Literal::BulkString(content),
                leftover,
            })
        }
        Some(b'*') => {
            let count_end = find_crlf(buffer).unwrap();
            let content_size = parse_number(buffer.get(1..count_end))?;
            if content_size < 0 {
                return Ok(ParseResult {
                    literal: Literal::NullArray,
                    leftover: None,
                });
            }
            let elements_start = count_end + 2;
            let mut leftover = buffer.get(elements_start..).map(|bytes| bytes.to_vec());
            let mut elements = Vec::new();
            for _ in 0..content_size {
                let result = try_parse_resp(&leftover.unwrap())?; //TODO: can we make this async and fire tokio spawn right away ?
                elements.push(result.literal);
                leftover = result.leftover;
            }
            Ok(ParseResult {
                literal: Literal::Array(elements),
                leftover,
            })
        }
        Some(e) => {
            dbg!(*e as char);
            todo!()
        }
        _ => todo!(),
    }
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

fn parse_number(digits: Option<&[u8]>) -> anyhow::Result<i64> {
    let digits = digits.ok_or_else(|| anyhow::anyhow!("range out of bounds"))?;
    Ok(std::str::from_utf8(digits)?.parse()?)
}

fn parse_string(bytes: Option<&[u8]>) -> anyhow::Result<String> {
    let bytes = bytes.ok_or_else(|| anyhow::anyhow!("range out of bounds"))?;
    Ok(std::str::from_utf8(bytes)?.to_string())
}

#[cfg(test)]
mod test {

    use std::assert_matches;

    use super::super::Literal;

    use super::ParseResult;

    #[test]
    fn test_string() {
        let result = dbg!(super::try_parse_resp(b"+OK\r\n").unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::String(ref s), leftover:_} if s == "OK")
        );
    }

    #[test]
    fn test_integer() {
        let result = dbg!(super::try_parse_resp(b":123456789\r\n").unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::Integer(ref s), leftover:_} if s == &123456789i64)
        );
    }

    #[test]
    fn test_bulk_string() {
        let result = dbg!(super::try_parse_resp(b"$5\r\nhello\r\n").unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::BulkString(ref s), leftover:_} if s == "hello")
        );
    }

    #[test]
    fn test_empty_string() {
        let result = dbg!(super::try_parse_resp(b"$0\r\n\r\n").unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::BulkString(ref s), leftover:_} if s == "")
        );
    }

    #[test]
    fn test_null_string() {
        let result = dbg!(super::try_parse_resp(b"$-1\r\n").unwrap());
        assert!(matches!(
            result,
            ParseResult {
                literal: Literal::NullBulkString,
                leftover: _
            }
        ));
    }

    #[test]
    fn test_bulk_long_string() {
        let result = super::try_parse_resp(b"$25\r\nhello\r\nmy brother whussup\r\n").unwrap();
        assert!(
            matches!(result, ParseResult {literal:  Literal::BulkString(ref s), leftover:_} if s == "hello\r\nmy brother whussup")
        );
    }

    #[test]
    fn test_array() {
        let result = dbg!(super::try_parse_resp(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").unwrap());

        match result.literal {
            Literal::Array(literals) => {
                let mut iterator = literals.iter();
                assert_matches!(iterator.next().unwrap(), Literal::BulkString(s) if "foo" == s);
                assert_matches!(iterator.next().unwrap(), Literal::BulkString(s) if "bar" == s);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_array_empty() {
        let result = dbg!(super::try_parse_resp(b"*0\r\n").unwrap());

        assert_matches!(result.literal, Literal::Array(literals) if literals.is_empty())
    }
    #[test]
    fn test_array_null() {
        let result = dbg!(super::try_parse_resp(b"*-1\r\n").unwrap());
        assert_matches!(
            result,
            ParseResult {
                literal: Literal::NullArray,
                leftover: _
            }
        );
    }

    #[test]
    fn test_array_nested() {
        let input = b"*3\r\n\
                  $3\r\nfoo\r\n\
                  *2\r\n:1\r\n$3\r\nbar\r\n\
                  :2\r\n";
        let result = dbg!(super::try_parse_resp(input).unwrap());

        match result.literal {
            Literal::Array(literals) => {
                let mut iterator = literals.iter();

                assert_matches!(iterator.next().unwrap(), Literal::BulkString(s) if "foo" == s);

                match iterator.next().unwrap() {
                    Literal::Array(literals) => {
                        let mut iterator = literals.iter();
                        assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 1 == *s);
                        assert_matches!(iterator.next().unwrap(), Literal::BulkString(s) if "bar" == *s);
                    }
                    _ => assert!(false),
                }

                assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 2 == *s);
            }
            _ => assert!(false),
        }
    }
}
