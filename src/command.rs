pub enum Command {
    Ping,
    Put(String, String),
    Get(String),
}

impl Command {
    pub fn try_parse(buffer: String) -> anyhow::Result<Self> {
        let sanitized: Vec<&str> = buffer.trim().splitn(3, ' ').collect(); //TODO: temporary since no command accepts more than 2 args
        match sanitized.as_slice() {
            ["PING"] => Ok(Command::Ping),
            ["GET", key] => Ok(Command::Get(key.to_string())),
            ["PUT", key, value] => Ok(Command::Put(key.to_string(), value.to_string())),
            _ => Err(anyhow::anyhow!(
                "Unable to understand such command {buffer}"
            )),
        }
    }
}
