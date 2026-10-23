use std::time::Instant;

use anyhow::bail;

use crate::{
    command::{Command, SetArgument},
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
            Command::Set(key, value, args) => {
                let nx = args.contains(&SetArgument::NX);
                let xx = args.contains(&SetArgument::XX); //TODO: XX & NX are exclusives
                let get = args.contains(&SetArgument::GET);
                let mut old_entry: Option<Entry> = None;
                if nx || xx || get {
                    match self.store.get(&key).await {
                        Ok(Some(_)) if nx => {
                            return Ok(Literal::NullBulkString);
                        }
                        Ok(None) if xx => return Ok(Literal::NullBulkString),
                        Ok(Some(entry)) if get => old_entry = Some(entry),
                        Err(_) => bail!("Cannot get an entry to check if it exists"),
                        _ => {}
                    }
                }
                self.store
                    .put(key.clone(), Entry::expiring(value.clone()))
                    .await?;
                if get {
                    return Ok(match old_entry {
                        Some(Entry {
                            value,
                            expires_at: _,
                        }) => Literal::BulkString(value),
                        None => Literal::NullBulkString,
                    });
                }
                Literal::String("OK".to_string())
            }
            Command::Get(key) => {
                if let Some(Entry {
                    value,
                    expires_at: _,
                }) = self.store.get(key.as_str()).await?
                {
                    Literal::BulkString(value)
                } else {
                    Literal::NullBulkString
                }
            }
            Command::Del(key) => self
                .store
                .delete(&key)
                .await?
                .map(|_| Literal::Integer(1))
                .unwrap_or_else(|| Literal::Integer(0)), //TODO: return number of elements removed
        })
    }
}
