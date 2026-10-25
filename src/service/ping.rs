use crate::protocol::Literal;

pub(super) fn execute(message: Option<String>) -> Literal {
    match message {
        Some(message) => Literal::BulkString(message),
        None => Literal::String("PONG".to_string()),
    }
}
