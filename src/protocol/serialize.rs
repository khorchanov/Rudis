use crate::protocol::Literal;

pub fn serialize(literal: Literal) -> String {
    match literal {
        Literal::NullBulkString => "$-1\r\n".to_owned(),
        Literal::NullArray => "*-1\r\n".to_owned(),
        Literal::String(string) => format!("+{string}\r\n"),
        Literal::BulkString(string) => {
            let length = string.len();
            format!("${length}\r\n{string}\r\n")
        }
        Literal::Integer(integer) => format!(":{integer}\r\n"),
        Literal::Array(literals) => {
            let length = literals.len();
            let mut result: Vec<String> = Vec::with_capacity(length);
            for element in literals {
                let serialized = serialize(element);
                result.push(serialized);
            }
            let joined = result.join("");
            format!("*{length}\r\n{joined}")
        }
    }
}
