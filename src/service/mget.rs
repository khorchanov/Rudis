use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &mut impl Store, items: Vec<String>) -> anyhow::Result<Literal> {
    let mut entries = Vec::with_capacity(items.len());
    for key in items {
        if let Some(Entry {
            value,
            expires_at_ms: _,
        }) = store.get(&key).await?
        {
            entries.push(Literal::BulkString(value));
        } else {
            entries.push(Literal::NullBulkString);
        }
    }
    Ok(Literal::Array(entries))
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

    fn mget(keys: &[&str]) -> Command {
        Command::Mget(keys.iter().map(|s| s.to_string()).collect())
    }

    async fn set_plain(executor: &mut Executor<impl crate::store::Store>, key: &str, value: &str) {
        executor
            .process_command_resp(set(key, value, None, None, false))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn returns_values_in_request_order() {
        let mut executor = Executor::default();
        set_plain(&mut executor, "a", "1").await;
        set_plain(&mut executor, "b", "2").await;

        let reply = executor
            .process_command_resp(mget(&["b", "a"]))
            .await
            .unwrap();

        assert_matches!(
            reply,
            Literal::Array(items) if matches!(
                items.as_slice(),
                [Literal::BulkString(x), Literal::BulkString(y)] if x == "2" && y == "1"
            )
        );
    }

    #[tokio::test]
    async fn missing_key_yields_nil_and_keeps_position() {
        let mut executor = Executor::default();
        set_plain(&mut executor, "a", "1").await;
        set_plain(&mut executor, "c", "3").await;

        let reply = executor
            .process_command_resp(mget(&["a", "missing", "c"]))
            .await
            .unwrap();

        assert_matches!(
            reply,
            Literal::Array(items) if matches!(
                items.as_slice(),
                [Literal::BulkString(a), Literal::NullBulkString, Literal::BulkString(c)]
                    if a == "1" && c == "3"
            )
        );
    }

    #[tokio::test]
    async fn all_keys_missing_yields_all_nils() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(mget(&["x", "y"]))
            .await
            .unwrap();

        assert_matches!(
            reply,
            Literal::Array(items) if matches!(
                items.as_slice(),
                [Literal::NullBulkString, Literal::NullBulkString]
            )
        );
    }

    #[tokio::test]
    async fn expired_key_yields_nil() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set(
                "gone",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(now_ms() - 1)),
                false,
            ))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(mget(&["gone"]))
            .await
            .unwrap();

        assert_matches!(
            reply,
            Literal::Array(items) if matches!(items.as_slice(), [Literal::NullBulkString])
        );
    }

    #[tokio::test]
    async fn duplicate_keys_are_returned_each_time() {
        let mut executor = Executor::default();
        set_plain(&mut executor, "a", "1").await;

        let reply = executor
            .process_command_resp(mget(&["a", "a"]))
            .await
            .unwrap();

        assert_matches!(
            reply,
            Literal::Array(items) if matches!(
                items.as_slice(),
                [Literal::BulkString(x), Literal::BulkString(y)] if x == "1" && y == "1"
            )
        );
    }
}
