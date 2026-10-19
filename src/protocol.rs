use anyhow::bail;
pub mod parse;

pub use parse::try_parse;

#[derive(Debug)]
pub enum Literal {
    //TODO: avoid String allocation by using &str
    Null,           //TODO: differentiate between null array and null bulk
    String(String), //TODO: Differentiate between bulk String and simple string
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
