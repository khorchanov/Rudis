use std::time::Instant;

use anyhow::Ok;

use crate::{
    command::Command,
    protocol::Literal,
    store::{Entry, Store},
};

pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock {}

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub struct Executor<S: Store, C: Clock> {
    pub store: S,
    pub clock: C,
}

impl<S: Store, C: Clock> Executor<S, C> {
    pub async fn process_command_resp(&mut self, command: Command) -> anyhow::Result<Literal> {
        Ok(match command {
            Command::Ping(Some(message)) | Command::Echo(message) => Literal::BulkString(message),
            Command::Ping(None) => Literal::String("PONG".to_string()),
            Command::Set(key, value) => {
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))
                    .await?;
                Literal::String("OK".to_string())
            }
            Command::Get(key) => match self.store.get(key.as_str()).await? {
                Some(entry) if self.clock.now() > entry.expires_at => {
                    let _ = self.store.delete(&key).await;
                    Literal::NullBulkString
                }
                Some(entry) => Literal::BulkString(entry.value),
                None => Literal::NullBulkString,
            },
            Command::Del(key) => self
                .store
                .delete(&key)
                .await?
                .map(|_| Literal::Integer(1))
                .unwrap_or_else(|| Literal::Integer(0)), //TODO: return number of elements removed
        })
    }
}
