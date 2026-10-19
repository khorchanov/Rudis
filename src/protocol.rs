use anyhow::bail;
mod parse;
mod serialize;

pub use parse::try_parse;

#[derive(Debug)]
//TODO: avoid String allocation by using &str
pub enum Literal {
    NullBulkString,
    NullArray,
    String(String),
    BulkString(String),
    Integer(usize),
    Array(Vec<Literal>),
}

impl Literal {
    pub fn as_string(self) -> anyhow::Result<String> {
        if let Self::String(content) = self {
            return Ok(content);
        }
        bail!("Literal Not a String")
    }
}
