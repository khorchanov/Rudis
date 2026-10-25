use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &mut impl Store, key: &str, by: i64) -> anyhow::Result<Literal> {
    Ok(
        if let Some(Entry {
            value,
            expires_at_ms,
        }) = store.get(key).await?
        {
            let old_value: i64 = value.parse()?;
            let new_value = old_value
                .checked_add(by)
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
                .put(key.to_string(), Entry::never_expires(by.to_string()))
                .await?;
            Literal::Integer(by)
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

    fn incr_by(key: &str, by: i64) -> Command {
        Command::IncrBy(key.to_string(), by)
    }

    #[tokio::test]
    async fn incr_by_creates_missing_key_with_the_increment() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(incr_by("k", 5))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(5));
        assert_eq!(stored(&executor, "k").await, Some(("5".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_by_adds_to_existing_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "10", None, None, false))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(incr_by("k", 7))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(17));
        assert_eq!(stored(&executor, "k").await, Some(("17".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_by_negative_amount_decrements() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "10", None, None, false))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(incr_by("k", -15))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(-5));
        assert_eq!(stored(&executor, "k").await, Some(("-5".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_by_zero_leaves_value_unchanged() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "3", None, None, false))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(incr_by("k", 0))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(3));
        assert_eq!(stored(&executor, "k").await, Some(("3".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_by_called_twice_accumulates() {
        let mut executor = Executor::default();

        executor
            .process_command_resp(incr_by("k", 4))
            .await
            .unwrap();
        let reply = executor
            .process_command_resp(incr_by("k", 6))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(10));
        assert_eq!(stored(&executor, "k").await, Some(("10".to_string(), None)));
    }

    #[tokio::test]
    async fn incr_by_fails_on_non_integer_and_keeps_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "abc", None, None, false))
            .await
            .unwrap();

        let result = executor.process_command_resp(incr_by("k", 1)).await;

        assert!(result.is_err());
        assert_eq!(
            stored(&executor, "k").await,
            Some(("abc".to_string(), None))
        );
    }

    #[tokio::test]
    async fn incr_by_fails_on_overflow_and_keeps_value() {
        let mut executor = Executor::default();
        let max = i64::MAX.to_string();
        executor
            .process_command_resp(set("k", &max, None, None, false))
            .await
            .unwrap();

        let result = executor.process_command_resp(incr_by("k", 1)).await;

        assert!(result.is_err());
        assert_eq!(stored(&executor, "k").await, Some((max, None)));
    }

    #[tokio::test]
    async fn incr_by_fails_on_underflow_and_keeps_value() {
        let mut executor = Executor::default();
        let min = i64::MIN.to_string();
        executor
            .process_command_resp(set("k", &min, None, None, false))
            .await
            .unwrap();

        let result = executor.process_command_resp(incr_by("k", -1)).await;

        assert!(result.is_err());
        assert_eq!(stored(&executor, "k").await, Some((min, None)));
    }

    #[tokio::test]
    async fn incr_by_keeps_existing_ttl() {
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

        executor
            .process_command_resp(incr_by("k", 9))
            .await
            .unwrap();

        assert_eq!(
            stored(&executor, "k").await,
            Some(("10".to_string(), Some(at)))
        );
    }

    #[tokio::test]
    async fn incr_by_treats_expired_key_as_missing() {
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

        let reply = executor
            .process_command_resp(incr_by("k", 5))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(5));
        assert_eq!(stored(&executor, "k").await, Some(("5".to_string(), None)));
    }
}
