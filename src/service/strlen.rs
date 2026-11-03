use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &impl Store, key: &str) -> anyhow::Result<Literal> {
    Ok(
        if let Some(Entry {
            value,
            expires_at_ms: _,
        }) = store.get(key).await?
        {
            Literal::Integer(i64::try_from(value.len())?)
        } else {
            Literal::Integer(0)
        },
    )
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        command::{Command, SetArgumentExpiryRule},
        protocol::Literal,
        service::{Executor, test_utils::set},
        utils::now_ms,
    };

    fn strlen(key: &str) -> Command {
        Command::StrLen(key.to_string())
    }

    #[tokio::test]
    async fn strlen_returns_length_of_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "hello", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(strlen("a")).await.unwrap();

        assert_matches!(reply, Literal::Integer(5));
    }

    #[tokio::test]
    async fn strlen_returns_zero_when_key_is_missing() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(strlen("missing"))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }

    #[tokio::test]
    async fn strlen_returns_zero_for_empty_value() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(strlen("a")).await.unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }

    #[tokio::test]
    async fn strlen_counts_bytes_not_chars() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "é", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(strlen("a")).await.unwrap();

        assert_matches!(reply, Literal::Integer(2));
    }

    #[tokio::test]
    async fn strlen_returns_zero_for_expired_key() {
        let mut executor = Executor::default();
        let past = now_ms() - 1_000;
        executor
            .process_command_resp(set(
                "a",
                "hello",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(past)),
                false,
            ))
            .await
            .unwrap();

        let reply = executor.process_command_resp(strlen("a")).await.unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }
}
