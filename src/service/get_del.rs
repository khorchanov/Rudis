use crate::{protocol::Literal, store::Store};

pub(super) async fn execute(store: &mut impl Store, key: &str) -> anyhow::Result<Literal> {
    Ok(if let Some(removed) = store.delete(key).await? {
        Literal::BulkString(removed.value)
    } else {
        Literal::NullBulkString
    })
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

    fn get_del(key: &str) -> Command {
        Command::GetDel(key.to_string())
    }

    #[tokio::test]
    async fn getdel_returns_value_and_removes_key() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "v", None, None, false))
            .await
            .unwrap();

        let reply = executor.process_command_resp(get_del("a")).await.unwrap();

        assert_matches!(reply, Literal::BulkString(v) if v == "v");
        assert_eq!(stored(&executor, "a").await, None);
    }

    #[tokio::test]
    async fn getdel_returns_null_when_key_is_missing() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(get_del("missing"))
            .await
            .unwrap();

        assert_matches!(reply, Literal::NullBulkString);
    }

    #[tokio::test]
    async fn getdel_second_call_returns_null() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "v", None, None, false))
            .await
            .unwrap();
        executor.process_command_resp(get_del("a")).await.unwrap();

        let reply = executor.process_command_resp(get_del("a")).await.unwrap();

        assert_matches!(reply, Literal::NullBulkString);
    }

    #[tokio::test]
    async fn getdel_does_not_touch_other_keys() {
        let mut executor = Executor::default();
        for key in ["a", "b"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }

        executor.process_command_resp(get_del("a")).await.unwrap();

        assert_eq!(stored(&executor, "b").await, Some(("v".to_string(), None)));
    }

    #[tokio::test]
    async fn getdel_returns_null_for_expired_key() {
        let mut executor = Executor::default();
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

        let reply = executor.process_command_resp(get_del("a")).await.unwrap();

        assert_matches!(reply, Literal::NullBulkString);
    }
}
