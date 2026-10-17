pub enum Command {
    Ping,
    Put(String, String),
    Get(String),
    Del(String),
}

impl Command {
    pub fn try_parse(buffer: &String) -> anyhow::Result<Self> {
        //TODO: support messages with spaces
        let sanitized: Vec<&str> = buffer.trim().split(' ').collect();
        let mut iterator: std::slice::Iter<'_, &str> = sanitized.iter();

        let command = match iterator.next() {
            Some(command) => command,
            None => todo!(),
        };

        match *command {
            "PING" => Ok(Command::Ping),
            "GET" => Ok(Command::Get(get_next(&mut iterator)?)),
            "DEL" => Ok(Command::Del(get_next(&mut iterator)?)),
            "PUT" => Ok(Command::Put(
                get_next(&mut iterator)?,
                get_next(&mut iterator)?,
            )),
            _ => Err(anyhow::anyhow!("Unable to understand such command")),
        }
    }
}

fn get_next(iterator: &mut std::slice::Iter<'_, &str>) -> anyhow::Result<String> {
    iterator
        .next()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to understand such command"))
}
