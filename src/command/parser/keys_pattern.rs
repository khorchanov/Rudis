use crate::protocol::Literal;

#[derive(Debug, PartialEq)]
pub enum GlobElement {
    Exact(char),
    AnyString,
    AnyCharacter,
    Set(Vec<char>),
    NegationSet(Vec<char>),
    Range(char, char),
}

pub(crate) fn parse(
    mut iterator: impl Iterator<Item = Literal>,
) -> anyhow::Result<Vec<GlobElement>> {
    let mut elements = Vec::new();
    let pattern: String = iterator
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .parse()?;
    let mut iterator = pattern.chars();
    while let Some(current) = iterator.next() {
        let element = match current {
            '*' => GlobElement::AnyString,
            '?' => GlobElement::AnyCharacter,
            '[' => {
                let set_patter: String = iterator.by_ref().take_while(|&c| c != ']').collect();
                if set_patter.starts_with('^') {
                    GlobElement::NegationSet(set_patter.chars().skip(1).collect())
                } else if set_patter.contains('-') {
                    let mut parts = set_patter.split('-');
                    let start = parts
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Wrong pattern for KEYS command"))?
                        .chars()
                        .last()
                        .ok_or_else(|| anyhow::anyhow!("Wrong pattern for KEYS command"))?; //TODO check size later
                    let end = parts
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Wrong pattern for KEYS command"))?
                        .chars()
                        .last()
                        .ok_or_else(|| anyhow::anyhow!("Wrong pattern for KEYS command"))?; //TODO check size later
                    GlobElement::Range(start, end)
                } else {
                    GlobElement::Set(set_patter.chars().collect())
                }
            }
            '\\' => {
                let c = iterator
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Wrong pattern for KEYS command"))?;
                GlobElement::Exact(c)
            }
            c => GlobElement::Exact(c),
        };
        elements.push(element);
    }
    Ok(elements)
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::GlobElement::*;
    use super::*;

    fn parse_pattern(pattern: &str) -> anyhow::Result<Vec<GlobElement>> {
        parse(iter::once(Literal::BulkString(pattern.to_string())))
    }

    #[test]
    fn missing_pattern_is_an_error() {
        assert!(parse(iter::empty()).is_err());
    }

    #[test]
    fn empty_pattern() {
        assert_eq!(parse_pattern("").unwrap(), vec![]);
    }

    #[test]
    fn literal_characters() {
        assert_eq!(
            parse_pattern("abc").unwrap(),
            vec![Exact('a'), Exact('b'), Exact('c')]
        );
    }

    #[test]
    fn wildcards() {
        assert_eq!(
            parse_pattern("h?llo*").unwrap(),
            vec![
                Exact('h'),
                AnyCharacter,
                Exact('l'),
                Exact('l'),
                Exact('o'),
                AnyString
            ]
        );
    }

    #[test]
    fn escaped_special_characters_are_literal() {
        assert_eq!(
            parse_pattern(r"h\*llo\?").unwrap(),
            vec![
                Exact('h'),
                Exact('*'),
                Exact('l'),
                Exact('l'),
                Exact('o'),
                Exact('?')
            ]
        );
    }

    #[test]
    fn escaped_backslash() {
        assert_eq!(parse_pattern(r"\\").unwrap(), vec![Exact('\\')]);
    }

    #[test]
    fn trailing_backslash_is_an_error() {
        assert!(parse_pattern("abc\\").is_err());
    }

    #[test]
    fn set() {
        assert_eq!(parse_pattern("[ae]").unwrap(), vec![Set(vec!['a', 'e'])]);
    }

    #[test]
    fn negation_set() {
        assert_eq!(parse_pattern("[^e]").unwrap(), vec![NegationSet(vec!['e'])]);
    }

    #[test]
    fn range() {
        assert_eq!(parse_pattern("[a-c]").unwrap(), vec![Range('a', 'c')]);
    }

    #[test]
    fn set_between_literals() {
        assert_eq!(
            parse_pattern("h[ae]llo").unwrap(),
            vec![
                Exact('h'),
                Set(vec!['a', 'e']),
                Exact('l'),
                Exact('l'),
                Exact('o')
            ]
        );
    }

    #[test]
    fn element_after_set_is_still_parsed() {
        assert_eq!(
            parse_pattern("[ab]*").unwrap(),
            vec![Set(vec!['a', 'b']), AnyString]
        );
    }
}
