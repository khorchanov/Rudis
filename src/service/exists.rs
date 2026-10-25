use crate::{protocol::Literal, store::Store};

pub(super) async fn execute(store: &impl Store, keys: Vec<String>) -> anyhow::Result<Literal> {
    let mut count = 0;
    for key in keys {
        if store.exists(&key).await? {
            count += 1;
        }
    }
    Ok(Literal::Integer(count))
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

    fn exists(keys: &[&str]) -> Command {
        Command::Exists(keys.iter().map(|key| key.to_string()).collect())
    }

    #[tokio::test]
    async fn exists_counts_existing_keys() {
        let mut executor = Executor::default();
        for key in ["a", "b"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }

        let reply = executor
            .process_command_resp(exists(&["a", "missing", "b"]))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(2));
    }

    #[tokio::test]
    async fn exists_returns_zero_when_no_key_exists() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(exists(&["x", "y"]))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }

    #[tokio::test]
    async fn exists_counts_duplicated_keys_each_time() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "v", None, None, false))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(exists(&["a", "a", "a"]))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(3));
    }

    #[tokio::test]
    async fn exists_returns_zero_after_key_is_deleted() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "v", None, None, false))
            .await
            .unwrap();
        executor
            .process_command_resp(Command::Del(vec!["a".to_string()]))
            .await
            .unwrap();

        let reply = executor.process_command_resp(exists(&["a"])).await.unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }

    #[tokio::test]
    async fn exists_ignores_expired_key() {
        let mut executor = Executor::default();
        // already in the past
        let past = now_ms() - 1_000;
        executor
            .process_command_resp(set(
                "a",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(past)),
                false,
            ))
            .await
            .unwrap();

        let reply = executor.process_command_resp(exists(&["a"])).await.unwrap();

        assert_matches!(reply, Literal::Integer(0));
    }
}
