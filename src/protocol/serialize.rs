use crate::protocol::Literal;

//TODO: to clean up
pub fn serialize(literal: Literal) -> Vec<u8> {
    match literal {
        Literal::NullBulkString => b"$-1\r\n".to_vec(),
        Literal::NullArray => b"*-1\r\n".to_vec(),
        Literal::String(string) => format!("+{string}\r\n").into_bytes(),
        Literal::BulkString(string) => {
            let length = string.len();
            format!("${length}\r\n{string}\r\n").into_bytes()
        }
        Literal::Integer(integer) => format!(":{integer}\r\n").into_bytes(),
        Literal::Array(literals) => {
            let mut result: Vec<u8> = Vec::new();
            result.push(b'*');
            result.extend_from_slice(literals.len().to_string().as_bytes());
            result.extend_from_slice(b"\r\n");
            for element in literals {
                let mut serialized = serialize(element);
                result.append(&mut serialized);
            }
            result
        }
    }
}
