use anyhow::bail;
mod parse;
mod serialize;

pub use parse::try_parse;
pub use serialize::serialize;

#[derive(Debug)]
//TODO: avoid String allocation by using &str
pub enum Literal {
    NullBulkString,
    NullArray,
    String(String),
    BulkString(String),
    Integer(i64),
    Array(Vec<Literal>),
}

impl Literal {
    pub fn as_string(self) -> anyhow::Result<String> {
        match self {
            Literal::String(content) | Literal::BulkString(content) => Ok(content),
            _ => bail!("Literal Not a String"),
        }
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::protocol::{
        serialize::{self, serialize},
        try_parse,
    };

    use super::Literal;

    #[test]
    fn test_full_round_null_bulk_string() {
        assert_matches!(
            try_parse(dbg!(&serialize(Literal::NullBulkString))).unwrap(),
            Literal::NullBulkString
        );
    }

    #[test]
    fn test_full_round_null_array() {
        assert_matches!(
            try_parse(dbg!(&serialize(Literal::NullArray))).unwrap(),
            Literal::NullArray
        );
    }

    #[test]
    fn test_full_round_string() {
        assert_matches!(
            try_parse(dbg!(&serialize(Literal::String("random".to_string())))).unwrap(),
            Literal::String(s) if s == "random"
        );
    }

    #[test]
    fn test_full_round_bulk_string() {
        assert_matches!(
            dbg!(try_parse(dbg!(&serialize(Literal::BulkString("random with \r\n spaces !".to_string())))).unwrap()),
            Literal::BulkString(s) if s == "random with \r\n spaces !"
        );
    }

    #[test]
    fn test_full_round_integer() {
        assert_matches!(
            try_parse(dbg!(&serialize(Literal::Integer(123456)))).unwrap(),
            Literal::Integer(s) if s == 123456
        );
    }

    #[test]
    fn test_full_round_array_nested() {
        let mut input: Vec<Literal> = Vec::new();
        input.push(Literal::BulkString("foo".to_string()));
        input.push(Literal::Array({
            let mut r = Vec::new();
            r.push(Literal::Integer(13));
            r.push(Literal::String("doo".to_string()));
            r
        }));
        input.push(Literal::Integer(0));

        let serialized = serialize(Literal::Array(input));
        dbg!(String::from_utf8_lossy(serialized.as_slice()));
        let parsed = try_parse(&serialized).unwrap();

        match parsed {
            Literal::Array(literals) => {
                let mut iterator = literals.iter();

                assert_matches!(iterator.next().unwrap(), Literal::BulkString(s) if "foo" == s);

                match iterator.next().unwrap() {
                    Literal::Array(literals) => {
                        let mut iterator = literals.iter();
                        assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 13 == *s);
                        assert_matches!(iterator.next().unwrap(), Literal::String(s) if "doo" == *s);
                    }
                    _ => assert!(false),
                }

                assert_matches!(iterator.next().unwrap(), Literal::Integer(s) if 0 == *s);
            }
            _ => assert!(false),
        }
    }
}
