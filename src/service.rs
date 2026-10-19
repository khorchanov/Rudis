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
    pub async fn process_command(&mut self, command: Command) -> anyhow::Result<Option<String>> {
        Ok(match command {
            Command::Ping => Some("PONG".to_string()),
            Command::Set(key, value) => {
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))
                    .await?;
                None
            }
            Command::Get(key) => match self.store.get(key.as_str()).await? {
                Some(entry) => {
                    if self.clock.now() > entry.expires_at {
                        let _ = self.store.delete(&key).await;
                        None
                    } else {
                        Some(entry.value)
                    }
                }
                None => None,
            },
            Command::Del(key) => self.store.delete(&key).await?.map(|entry| entry.value),
        })
    }

    pub async fn process_command_resp(&mut self, command: Command) -> anyhow::Result<Literal> {
        Ok(match command {
            Command::Ping => Literal::String("PONG".to_string()),
            Command::Set(key, value) => {
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))
                    .await?;
                Literal::String("OK".to_string())
            }
            Command::Get(key) => match self.store.get(key.as_str()).await? {
                Some(entry) if self.clock.now() > entry.expires_at => {
                    let _ = self.store.delete(&key).await;
                    Literal::Null
                }
                Some(entry) => Literal::String(entry.value),
                None => Literal::Null,
            },
            Command::Del(key) => self
                .store
                .delete(&key)
                .await?
                .map(|entry| Literal::String(entry.value))
                .unwrap_or_else(|| Literal::Null),
        })
    }
}
