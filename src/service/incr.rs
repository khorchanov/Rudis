use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &mut impl Store, key: &str) -> anyhow::Result<Literal> {
    Ok(
        if let Some(Entry {
            value,
            expires_at_ms,
        }) = store.get(key).await?
        {
            let old_value: i64 = value.parse()?;
            let new_value = old_value
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("This will overflow"))?;
            store
                .put(
                    key.to_string(),
                    Entry::may_expires_in_millis(new_value.to_string(), expires_at_ms),
                )
                .await?;
            Literal::Integer(new_value)
        } else {
            store
                .put(key.to_string(), Entry::never_expires("1".to_string()))
                .await?;
            Literal::Integer(1)
        },
    )
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        command::{Command, SetArgumentExpiryRule},
        protocol::Literal,
        service::{
            Executor,
            test_utils::{set, stored},
        },
        utils::now_ms,
    };

    fn incr(key: &str) -> Command {
        Command::Incr(key.to_string())
    }

    #[tokio::test]
    async fn incr_creates_missing_key_at_one() {
        let mut executor = Executor::default();

        let reply = executor.process_command_resp(incr("k")).await.unwrap();

        assert_matches!(reply, Literal::Integer(1));
        assert_eq!(stored(&executor, "k").await, Some(("1".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_increments_existing_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "5", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(incr("k")).await.unwrap();

        assert_matches!(reply, Literal::Integer(6));
        assert_eq!(stored(&executor, "k").await, Some(("6".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_called_twice_counts_up() {
        let mut executor = Executor::default();

        executor.process_command_resp(incr("k")).await.unwrap();
        let reply = executor.process_command_resp(incr("k")).await.unwrap();

        assert_matches!(reply, Literal::Integer(2));
        assert_eq!(stored(&executor, "k").await, Some(("2".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_negative_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "-3", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(incr("k")).await.unwrap();

        assert_matches!(reply, Literal::Integer(-2));
        assert_eq!(stored(&executor, "k").await, Some(("-2".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_fails_on_non_integer_and_keeps_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "abc", None, None, false))
            .await
            .unwrap();

        let result = executor.process_command_resp(incr("k")).await;

        assert!(result.is_err());
        assert_eq!(
            stored(&executor, "k").await,
            Some(("abc".to_string(), None))
        );
    }

    #[tokio::test]
    async fn incr_fails_on_overflow_and_keeps_value() {
        let mut executor = Executor::default();
        let max = i64::MAX.to_string();
        executor
            .process_command_resp(set("k", &max, None, None, false))
            .await
            .unwrap();

        let result = executor.process_command_resp(incr("k")).await;

        assert!(result.is_err());
        assert_eq!(stored(&executor, "k").await, Some((max, None)));
    }

    #[tokio::test]
    async fn incr_keeps_existing_ttl() {
        let mut executor = Executor::default();
        let at = now_ms() + 60_000;
        executor
            .process_command_resp(set(
                "k",
                "1",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(at)),
                false,
            ))
            .await
            .unwrap();

        executor.process_command_resp(incr("k")).await.unwrap();

        assert_eq!(
            stored(&executor, "k").await,
            Some(("2".to_string(), Some(at)))
        );
    }

    #[tokio::test]
    async fn incr_treats_expired_key_as_missing() {
        let mut executor = Executor::default();
        let past = now_ms() - 1_000;
        executor
            .process_command_resp(set(
                "k",
                "41",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(past)),
                false,
            ))
            .await
            .unwrap();

        let reply = executor.process_command_resp(incr("k")).await.unwrap();

        assert_matches!(reply, Literal::Integer(1));
        assert_eq!(stored(&executor, "k").await, Some(("1".to_string(), None)));
    }
}
