use crate::protocol::Literal;

pub(super) fn execute(message: String) -> Literal {
    Literal::BulkString(message)
}
