use crate::{
    command::TtlReturnUnit,
    protocol::Literal,
    store::{Entry, Store},
    utils::now_ms,
};

pub(super) async fn execute(
    store: &mut impl Store,
    key: &str,
    unit: TtlReturnUnit,
) -> anyhow::Result<Literal> {
    Ok(
        if let Some(Entry {
            value: _,
            expires_at_ms,
        }) = store.get(key).await?
        {
            if let Some(expiry_at_ms) = expires_at_ms {
                let remaining_ms = expiry_at_ms.saturating_sub(now_ms());
                match unit {
                    TtlReturnUnit::Seconds => Literal::Integer((remaining_ms / 1000) as i64),
                    TtlReturnUnit::Millis => Literal::Integer(remaining_ms as i64),
                }
            } else {
                Literal::Integer(-1)
            }
        } else {
            Literal::Integer(-2)
        },
    )
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        command::{Command, SetArgumentExpiryRule, TtlReturnUnit},
        protocol::Literal,
        service::{Executor, test_utils::set},
        utils::now_ms,
    };

    fn ttl(key: &str, unit: TtlReturnUnit) -> Command {
        Command::Ttl(key.to_string(), unit)
    }

    #[tokio::test]
    async fn ttl_missing_key_returns_minus_two() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(ttl("missing", TtlReturnUnit::Seconds))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(-2));
    }

    #[tokio::test]
    async fn pttl_missing_key_returns_minus_two() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(ttl("missing", TtlReturnUnit::Millis))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(-2));
    }

    #[tokio::test]
    async fn ttl_key_without_expiry_returns_minus_one() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v", None, None, false))
            .await
            .unwrap();

        let seconds = executor
            .process_command_resp(ttl("k", TtlReturnUnit::Seconds))
            .await
            .unwrap();
        let millis = executor
            .process_command_resp(ttl("k", TtlReturnUnit::Millis))
            .await
            .unwrap();

        assert_matches!(seconds, Literal::Integer(-1));
        assert_matches!(millis, Literal::Integer(-1));
    }

    #[tokio::test]
    async fn ttl_expired_key_returns_minus_two() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set(
                "k",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(now_ms() - 1_000)),
                false,
            ))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(ttl("k", TtlReturnUnit::Millis))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(-2));
    }

    #[tokio::test]
    async fn pttl_returns_remaining_milliseconds() {
        let mut executor = Executor::default();
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

        let reply = executor
            .process_command_resp(ttl("k", TtlReturnUnit::Millis))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(ms) if (9_000..=10_000).contains(&ms));
    }

    #[tokio::test]
    async fn ttl_returns_remaining_seconds() {
        let mut executor = Executor::default();
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

        let reply = executor
            .process_command_resp(ttl("k", TtlReturnUnit::Seconds))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(s) if (9..=10).contains(&s));
    }
}
