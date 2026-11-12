use crate::protocol::Literal;

pub fn execute(protocol: Option<i64>) -> anyhow::Result<Literal> {
    match protocol {
        None | Some(2) => Ok(hello_reply()),
        Some(_) => anyhow::bail!("NOPROTO unsupported protocol version"),
    }
}

fn hello_reply() -> Literal {
    let bulk = |s: &str| Literal::BulkString(s.to_string());

    Literal::Array(vec![
        bulk("server"),  bulk("redis"),
        bulk("version"), bulk("7.2.0"),
        bulk("proto"),   Literal::Integer(2),
        bulk("id"),      Literal::Integer(1),
        bulk("mode"),    bulk("standalone"),
        bulk("role"),    bulk("master"),
        bulk("modules"), Literal::Array(vec![]),
    ])
}