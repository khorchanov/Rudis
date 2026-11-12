use crate::protocol::Literal;

pub fn execute() -> Literal {
    Literal::BulkString(
        [
            "# Server",
            "redis_version:7.2.0",
            "redis_mode:standalone",
            "",
            "# Replication",
            "role:master",
            "",
        ]
        .join("\r\n"),
    )
}
