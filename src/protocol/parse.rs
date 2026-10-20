use super::Literal;

/// One shot parse of the buffer and return processed bytes (buffer could need more passes to parse next)
/// Returns Ok(None) if data is just incomplete
/// Returns Error if wrong format or protocol error

pub fn try_parse_resp(buf: &[u8]) -> anyhow::Result<Option<(Literal, usize)>> {
    let Some(&prefix) = buf.first() else {
        return Ok(None);
    };
    let Some(end) = find_crlf(buf) else {
        return Ok(None);
    };
    let header = end + 2;

    match prefix {
        b':' => {
            let n = parse_number(buf.get(1..end))?;
            Ok(Some((Literal::Integer(n), header)))
        }
        b'+' => {
            let s = parse_string(buf.get(1..end))?;
            Ok(Some((Literal::String(s), header)))
        }
        b'$' => {
            let len = parse_number(buf.get(1..end))?;
            if len == -1 {
                return Ok(Some((Literal::NullBulkString, header)));
            }
            let len = usize::try_from(len)?;
            let total = header + len + 2;
            if buf.len() < total {
                return Ok(None);
            }
            if &buf[header + len..total] != b"\r\n" {
                anyhow::bail!("bulk string missing CRLF");
            }
            let s = parse_string(buf.get(header..header + len))?;
            Ok(Some((Literal::BulkString(s), total)))
        }
        b'*' => {
            let count = parse_number(buf.get(1..end))?;
            if count == -1 {
                return Ok(Some((Literal::NullArray, header)));
            }
            let count = usize::try_from(count)?;
            let mut pos = header;
            let mut elements = Vec::new();
            for _ in 0..count {
                let Some((lit, used)) = try_parse_resp(&buf[pos..])? else {
                    return Ok(None);
                };
                elements.push(lit);
                pos += used;
            }
            Ok(Some((Literal::Array(elements), pos)))
        }
        other => anyhow::bail!("unknown type byte: {}", other as char),
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

    #[test]
    fn test_string() {
        let result = dbg!(super::try_parse_resp(b"+OK\r\n").unwrap());
        assert!(matches!(result, Some((Literal::String(ref s), _)) if s == "OK"));
    }

    #[test]
    fn test_integer() {
        let result = dbg!(super::try_parse_resp(b":123456789\r\n").unwrap());
        assert!(matches!(result, Some((Literal::Integer(ref s), _)) if s == &123456789i64));
    }

    #[test]
    fn test_bulk_string() {
        let result = dbg!(super::try_parse_resp(b"$5\r\nhello\r\n").unwrap());
        assert!(matches!(result, Some((Literal::BulkString(ref s), _)) if s == "hello"));
    }

    #[test]
    fn test_empty_string() {
        let result = dbg!(super::try_parse_resp(b"$0\r\n\r\n").unwrap());
        assert!(matches!(result, Some((Literal::BulkString(ref s), _)) if s == ""));
    }

    #[test]
    fn test_null_string() {
        let result = dbg!(super::try_parse_resp(b"$-1\r\n").unwrap());
        assert!(matches!(result, Some((Literal::NullBulkString, _))));
    }

    #[test]
    fn test_bulk_long_string() {
        let result = super::try_parse_resp(b"$25\r\nhello\r\nmy brother whussup\r\n").unwrap();
        assert!(
            matches!(result, Some((Literal::BulkString(ref s), _)) if s == "hello\r\nmy brother whussup")
        );
    }

    #[test]
    fn test_array() {
        let literal = dbg!(super::try_parse_resp(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").unwrap())
            .unwrap()
            .0;

        match literal {
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
        let literal = dbg!(super::try_parse_resp(b"*0\r\n").unwrap()).unwrap().0;

        assert_matches!(literal, Literal::Array(literals) if literals.is_empty())
    }
    #[test]
    fn test_array_null() {
        let result = dbg!(super::try_parse_resp(b"*-1\r\n").unwrap()).unwrap().0;
        assert_matches!(result, Literal::NullArray,);
    }

    #[test]
    fn test_array_nested() {
        let input = b"*3\r\n\
                  $3\r\nfoo\r\n\
                  *2\r\n:1\r\n$3\r\nbar\r\n\
                  :2\r\n";
        let literal = dbg!(super::try_parse_resp(input).unwrap()).unwrap().0;

        match literal {
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
