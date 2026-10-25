use anyhow::bail;

use crate::{
    command::{Command, SetArgumentExpiryRule, SetArgumentRule, SetCommand},
    protocol::Literal,
    store::{Entry, Store},
    utils::now_ms,
};

pub trait Clock {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock {}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        now_ms()
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
            Command::Set(SetCommand {
                key,
                value,
                return_old_value,
                set_rule,
                expiry_rule,
            }) => {
                if let Some(rule) = dbg!(set_rule) {
                    match (rule, self.store.get(&key).await) {
                        (SetArgumentRule::NX, Ok(Some(_))) => {
                            return Ok(Literal::NullBulkString);
                        }
                        (SetArgumentRule::XX, Ok(None)) => {
                            return Ok(Literal::NullBulkString);
                        }
                        (_, Err(_)) => bail!("Cannot get an entry to check if it exists"),
                        _ => {}
                    }
                }
                let entry = match dbg!(expiry_rule) {
                    Some(SetArgumentExpiryRule::ExpiresAtMs(at)) => {
                        Entry::expiring_at_millis(value, at)
                    }
                    Some(SetArgumentExpiryRule::ExpiresAfterMs(after)) => {
                        Entry::expiring_at_millis(value, now_ms() + after)
                    }
                    Some(SetArgumentExpiryRule::KeepTtl) => {
                        if let Ok(Some(Entry {
                            value: _,
                            expires_at_ms: Some(expires_at_ms),
                        })) = self.store.get(&key).await
                        {
                            Entry::expiring_at_millis(value, expires_at_ms)
                        } else {
                            Entry::never_expires(value)
                        }
                    }
                    None => Entry::never_expires(value),
                };
                let old_entry = self.store.put(key.clone(), dbg!(entry)).await?;
                if return_old_value {
                    return Ok(match old_entry {
                        Some(Entry {
                            value,
                            expires_at_ms: _,
                        }) => Literal::BulkString(value),
                        None => Literal::NullBulkString,
                    });
                }
                Literal::String("OK".to_string())
            }
            Command::Get(key) => {
                if let Some(Entry {
                    value,
                    expires_at_ms: _,
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
