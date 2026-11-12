use anyhow::bail;

use crate::protocol::Literal;

pub fn execute(args: Vec<String>) -> anyhow::Result<Literal> {
    let Some((subcommand, rest)) = args.split_first() else {
        bail!("wrong number of arguments for 'client' command");
    };
    match (subcommand.to_uppercase().as_str(), rest) {
        ("SETNAME", [_name]) => Ok(Literal::String("OK".to_string())),
        ("SETINFO", [_attribute, _value]) => Ok(Literal::String("OK".to_string())),
        ("GETNAME", []) => Ok(Literal::NullBulkString),
        ("ID", []) => Ok(Literal::Integer(1)),
        (sub, _) => bail!("unknown subcommand or wrong number of arguments for 'CLIENT {sub}'"),
    }
}
