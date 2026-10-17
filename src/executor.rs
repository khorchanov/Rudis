use std::time::Instant;

use anyhow::Ok;

use crate::{
    command::Command,
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
    pub async fn process(&mut self, command: Command) -> anyhow::Result<Option<String>> {
        Ok(match command {
            Command::Ping => Some("PONG".to_string()),
            Command::Put(key, value) => {
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
}
