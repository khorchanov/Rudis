use std::time::Instant;

use crate::store::{Entry, Store};

pub enum Command {
    Ping,
    Put(String, String),
    Get(String),
}

pub trait Clock {
    fn now() -> Instant;
}

pub struct SystemClock {}

impl Clock for SystemClock {
    fn now() -> Instant {
        Instant::now()
    }
}

pub struct Executor<S: Store, C: Clock> {
    pub store: S,
    pub clock: C,
}

impl<S: Store, C: Clock> Executor<S, C> {

    pub fn process(&mut self, command: Command) -> anyhow::Result<Option<String>> {
        Ok(match command {
            Command::Ping => Some("PONG".to_string()),
            Command::Put(key, value) => {
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))?;
                None
            }
            Command::Get(key) => self.store.get(key.as_str())?.map(|entry| entry.value),
        })
    }
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
