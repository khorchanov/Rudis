use anyhow::bail;

use super::Literal;


pub fn try_parse(buffer: &String) -> anyhow::Result<Literal> {
    try_parse_resp(buffer)?.strip()
}

#[derive(Debug)]
struct ParseResult {
    literal: Literal,
    leftover: Option<String>,
}

impl ParseResult {
    fn strip(self) -> anyhow::Result<Literal> {
        if let Some(content) = self.leftover
            && content.is_empty()
        {
            return Ok(self.literal);
        }
        bail!("Unfinished Literal")
    }
}

fn try_parse_resp(buffer: &String) -> anyhow::Result<ParseResult> {
    //TODO: support bytes input
    let mut chars = buffer.chars();
    match chars.nth(0) {
        Some(':') => {
            let end = buffer.find("\r\n").unwrap();
            let content: usize = buffer.get(1..end).unwrap().parse().unwrap();
            let leftover = buffer.get(end + 2..).map(String::from);
            return Ok(ParseResult {
                literal: Literal::Integer(content),
                leftover,
            });
        }
        Some('+') => {
            let end = buffer.find("\r\n").unwrap();
            let content = buffer.get(1..end).unwrap();
            let leftover = buffer.get(end + 2..).map(String::from);
            return Ok(ParseResult {
                literal: Literal::String(content.to_owned()),
                leftover,
            });
        }
        Some('$') => {
            let size_end = buffer.find("\r\n").unwrap();
            let content_size: isize = buffer.get(1..size_end).unwrap().parse().unwrap();
            if content_size < 0 {
                return Ok(ParseResult {
                    literal: Literal::Null,
                    leftover: None,
                });
            }
            let content_start = size_end + 2;
            let content = buffer
                .get(content_start..content_start + content_size as usize)
                .unwrap();
            let element_end = content_start + content_size as usize + 2;
            let leftover = buffer.get(element_end..).map(String::from);
            return Ok(ParseResult {
                literal: Literal::String(content.to_owned()),
                leftover,
            });
        }
        Some('*') => {
            let count_end = buffer.find("\r\n").unwrap();
            let content_size: isize = buffer.get(1..count_end).unwrap().parse().unwrap();
            if content_size < 0 {
                return Ok(ParseResult {
                    literal: Literal::Null,
                    leftover: None,
                });
            }
            let elements_start = count_end + 2;
            let mut leftover = buffer.get(elements_start..).map(String::from);
            let mut elements = Vec::new();
            for _ in 0..content_size {
                let result = try_parse_resp(&leftover.unwrap())?; //TODO: can we make this async and fire tokio spawn right away ?
                elements.push(result.literal);
                leftover = result.leftover;
            }
            return Ok(ParseResult {
                literal: Literal::Array(elements),
                leftover,
            });
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod test {

    use std::assert_matches;

    use super::super::Literal;

use super::{ParseResult};

    #[test]
    fn test_string() {
        let result = dbg!(super::try_parse_resp(&"+OK\r\n".to_string()).unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::String(ref s), leftover:_} if s == "OK")
        );
    }

    #[test]
    fn test_integer() {
        let result = dbg!(super::try_parse_resp(&":123456789\r\n".to_string()).unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::Integer(ref s), leftover:_} if s == &123456789usize)
        );
    }

    #[test]
    fn test_bulk_string() {
        let result = dbg!(super::try_parse_resp(&"$5\r\nhello\r\n".to_string()).unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::String(ref s), leftover:_} if s == "hello")
        );
    }

    #[test]
    fn test_empty_string() {
        let result = dbg!(super::try_parse_resp(&"$0\r\n\r\n".to_string()).unwrap());
        assert!(
            matches!(result, ParseResult{literal : Literal::String(ref s), leftover:_} if s == "")
        );
    }

    #[test]
    fn test_null_string() {
        let result = dbg!(super::try_parse_resp(&"$-1\r\n".to_string()).unwrap());
        assert!(matches!(
            result,
            ParseResult {
                literal: Literal::Null,
                leftover: _
            }
        ));
    }

    #[test]
    fn test_bulk_long_string() {
        let result =
            super::try_parse_resp(&"$25\r\nhello\r\nmy brother whussup\r\n".to_string()).unwrap();
        assert!(
            matches!(result, ParseResult {literal:  Literal::String(ref s), leftover:_} if s == "hello\r\nmy brother whussup")
        );
    }

    #[test]
    fn test_array() {
        let result =
            dbg!(super::try_parse_resp(&"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".to_string()).unwrap());

        match result.literal {
            Literal::Array(literals) => {
                let mut iterator = literals.iter();
                assert_matches!(iterator.next().unwrap(), Literal::String(s) if "foo" == s);
                assert_matches!(iterator.next().unwrap(), Literal::String(s) if "bar" == s);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_array_empty() {
        let result = dbg!(super::try_parse_resp(&"*0\r\n".to_string()).unwrap());

        assert_matches!(result.literal, Literal::Array(literals) if literals.is_empty())
    }
    #[test]
    fn test_array_null() {
        let result = dbg!(super::try_parse_resp(&"*-1\r\n".to_string()).unwrap());
        assert_matches!(
            result,
            ParseResult {
                literal: Literal::Null,
                leftover: _
            }
        );
    }

    #[test]
    fn test_array_nested() {
        let input = "*3\r\n\
                  $3\r\nfoo\r\n\
                  *2\r\n:1\r\n$3\r\nbar\r\n\
                  :2\r\n"
            .to_string();
        let result = dbg!(super::try_parse_resp(&input).unwrap());

        match result.literal {
            Literal::Array(literals) => {
                let mut iterator = literals.iter();

                assert_matches!(iterator.next().unwrap(), Literal::String(s) if "foo" == s);

                match iterator.next().unwrap() {
                    Literal::Array(literals) => {
                        let mut iterator = literals.iter();
                        assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 1 == *s);
                        assert_matches!(iterator.next().unwrap(), Literal::String(s) if "bar" == *s);
                    }
                    _ => assert!(false),
                }

                assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 2 == *s);
            }
            _ => assert!(false),
        }
    }
}
