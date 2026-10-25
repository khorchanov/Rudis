use anyhow::bail;

use crate::{
    HashMapStore,
    command::{Command, SetArgumentExpiryRule, SetArgumentRule, SetCommand},
    protocol::Literal,
    store::{Entry, Store, SystemClock},
    utils::now_ms,
};

pub struct Executor<S: Store> {
    pub store: S,
}

impl Default for Executor<HashMapStore<SystemClock>> {
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}

impl<S: Store> Executor<S> {
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
            Command::Del(keys) => Literal::Integer(self.store.delete_many(keys).await? as i64),
        })
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        command::{Command, SetArgumentExpiryRule, SetArgumentRule, SetCommand},
        protocol::Literal,
        service::Executor,
        store::Store,
        utils::now_ms,
    };

    fn set(
        key: &str,
        value: &str,
        set_rule: Option<SetArgumentRule>,
        expiry_rule: Option<SetArgumentExpiryRule>,
        return_old_value: bool,
    ) -> Command {
        Command::Set(SetCommand {
            key: key.to_string(),
            value: value.to_string(),
            return_old_value,
            set_rule,
            expiry_rule,
        })
    }

    async fn stored(executor: &Executor<impl Store>, key: &str) -> Option<(String, Option<u64>)> {
        executor
            .store
            .get(key)
            .await
            .unwrap()
            .map(|entry| (entry.value, entry.expires_at_ms))
    }

    #[tokio::test]
    async fn plain_set_overwrites_and_replies_ok() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v1", None, None, false))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(set("k", "v2", None, None, false))
            .await
            .unwrap();

        assert_matches!(reply, Literal::String(s) if s == "OK");
        assert_eq!(stored(&executor, "k").await, Some(("v2".to_string(), None)));
    }

    #[tokio::test]
    async fn nx_sets_when_key_is_missing() {
        let mut executor = Executor::default();
        let reply = executor
            .process_command_resp(set("k", "v", Some(SetArgumentRule::NX), None, false))
            .await
            .unwrap();

        assert_matches!(reply, Literal::String(s) if s == "OK");
        assert_eq!(stored(&executor, "k").await, Some(("v".to_string(), None)));
    }

    #[tokio::test]
    async fn nx_does_not_overwrite_existing_key() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "old", None, None, false))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(set("k", "new", Some(SetArgumentRule::NX), None, false))
            .await
            .unwrap();

        assert_matches!(reply, Literal::NullBulkString);
        assert_eq!(
            stored(&executor, "k").await,
            Some(("old".to_string(), None))
        );
    }

    #[tokio::test]
    async fn xx_does_not_set_missing_key() {
        let mut executor = Executor::default();
        let reply = executor
            .process_command_resp(set("k", "v", Some(SetArgumentRule::XX), None, false))
            .await
            .unwrap();

        assert_matches!(reply, Literal::NullBulkString);
        assert_eq!(stored(&executor, "k").await, None);
    }

    #[tokio::test]
    async fn xx_overwrites_existing_key() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "old", None, None, false))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(set("k", "new", Some(SetArgumentRule::XX), None, false))
            .await
            .unwrap();

        assert_matches!(reply, Literal::String(s) if s == "OK");
        assert_eq!(
            stored(&executor, "k").await,
            Some(("new".to_string(), None))
        );
    }

    #[tokio::test]
    async fn get_returns_old_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "old", None, None, false))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(set("k", "new", None, None, true))
            .await
            .unwrap();

        assert_matches!(reply, Literal::BulkString(s) if s == "old");
        assert_eq!(
            stored(&executor, "k").await,
            Some(("new".to_string(), None))
        );
    }

    #[tokio::test]
    async fn get_returns_nil_when_key_was_missing() {
        let mut executor = Executor::default();
        let reply = executor
            .process_command_resp(set("k", "new", None, None, true))
            .await
            .unwrap();

        assert_matches!(reply, Literal::NullBulkString);
        assert_eq!(
            stored(&executor, "k").await,
            Some(("new".to_string(), None))
        );
    }

    #[tokio::test]
    async fn expires_after_ms_sets_expiry_relative_to_now() {
        let mut executor = Executor::default();
        let before = now_ms();
        executor
            .process_command_resp(set(
                "k",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAfterMs(10_000)),
                false,
            ))
            .await
            .unwrap();
        let after = now_ms();

        let (_, expires_at) = stored(&executor, "k").await.unwrap();
        let expires_at = expires_at.expect("entry should expire");
        assert!(
            (before + 10_000..=after + 10_000).contains(&expires_at),
            "expires_at = {expires_at}"
        );
    }

    #[tokio::test]
    async fn expires_at_ms_sets_absolute_expiry() {
        let mut executor = Executor::default();
        let at = now_ms() + 60_000;
        executor
            .process_command_resp(set(
                "k",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(at)),
                false,
            ))
            .await
            .unwrap();

        assert_eq!(
            stored(&executor, "k").await,
            Some(("v".to_string(), Some(at)))
        );
    }

    #[tokio::test]
    async fn plain_set_clears_existing_ttl() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set(
                "k",
                "v1",
                None,
                Some(SetArgumentExpiryRule::ExpiresAfterMs(10_000)),
                false,
            ))
            .await
            .unwrap();
        executor
            .process_command_resp(set("k", "v2", None, None, false))
            .await
            .unwrap();

        assert_eq!(stored(&executor, "k").await, Some(("v2".to_string(), None)));
    }

    #[tokio::test]
    async fn keep_ttl_preserves_existing_expiry() {
        let mut executor = Executor::default();
        let at = now_ms() + 60_000;
        executor
            .process_command_resp(set(
                "k",
                "v1",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(at)),
                false,
            ))
            .await
            .unwrap();
        executor
            .process_command_resp(set(
                "k",
                "v2",
                None,
                Some(SetArgumentExpiryRule::KeepTtl),
                false,
            ))
            .await
            .unwrap();

        assert_eq!(
            stored(&executor, "k").await,
            Some(("v2".to_string(), Some(at)))
        );
    }

    #[tokio::test]
    async fn keep_ttl_on_key_without_ttl_never_expires() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v1", None, None, false))
            .await
            .unwrap();
        executor
            .process_command_resp(set(
                "k",
                "v2",
                None,
                Some(SetArgumentExpiryRule::KeepTtl),
                false,
            ))
            .await
            .unwrap();

        assert_eq!(stored(&executor, "k").await, Some(("v2".to_string(), None)));
    }

    #[tokio::test]
    async fn xx_with_expiry_and_get() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "old", None, None, false))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(set(
                "k",
                "new",
                Some(SetArgumentRule::XX),
                Some(SetArgumentExpiryRule::ExpiresAfterMs(10_000)),
                true,
            ))
            .await
            .unwrap();

        assert_matches!(reply, Literal::BulkString(s) if s == "old");
        let (value, expires_at) = stored(&executor, "k").await.unwrap();
        assert_eq!(value, "new");
        assert!(expires_at.is_some());
    }

    #[tokio::test]
    async fn del_many_returns_number_of_deleted_keys() {
        let mut executor = Executor::default();
        for key in ["a", "b", "c"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }

        let keys = ["a", "b", "missing", "c"].map(String::from).to_vec();
        let reply = executor
            .process_command_resp(Command::Del(keys))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(3));
        for key in ["a", "b", "c"] {
            assert_eq!(stored(&executor, key).await, None);
        }
    }

    #[tokio::test]
    async fn del_many_returns_zero_when_no_key_exists() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("untouched", "v", None, None, false))
            .await
            .unwrap();

        let keys = ["x", "y", "z"].map(String::from).to_vec();
        let reply = executor
            .process_command_resp(Command::Del(keys))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(0));
        assert_eq!(
            stored(&executor, "untouched").await,
            Some(("v".to_string(), None))
        );
    }
}
