use std::time::Instant;

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
    pub fn process(&mut self, command: Command) -> anyhow::Result<Option<String>> {
        Ok(match command {
            Command::Ping => Some("PONG".to_string()),
            Command::Put(key, value) => {
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))?;
                None
            }
            Command::Get(key) => self
                .store
                .get(key.as_str())?
                .filter(|entry| {
                    if self.clock.now() > entry.expires_at {
                        let _ = self.store.delete(&key);
                        return false;
                    }
                    true
                })
                .map(|entry| entry.value),
        })
    }
}
