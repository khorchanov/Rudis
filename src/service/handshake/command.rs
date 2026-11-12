use anyhow::bail;

use crate::protocol::Literal;

pub fn execute(args: Vec<String>) -> anyhow::Result<Literal> {
    let Some(subcommand) = args.first() else {
        return Ok(Literal::Array(vec![]));
    };
    match subcommand.to_uppercase().as_str() {
        "COUNT" => Ok(Literal::Integer(0)),
        "DOCS" | "INFO" => Ok(Literal::Array(vec![])),
        sub => bail!("unknown subcommand '{sub}'"),
    }
}
