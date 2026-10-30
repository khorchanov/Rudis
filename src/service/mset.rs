use anyhow::bail;

use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &mut impl Store, items: Vec<String>) -> anyhow::Result<Literal> {
    if items.is_empty() || items.len() % 2 != 0 {
        bail!("wrong number of arguments for 'mset' command");
    }
    let mut items = items.into_iter();
    let mut entries = Vec::with_capacity(items.len() / 2);
    while let (Some(key), Some(value)) = (items.next(), items.next()) {
        entries.push((key, Entry::never_expires(value)));
    }
    store.put_many(entries).await?;
    Ok(Literal::String("OK".to_string()))
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        command::Command,
        protocol::Literal,
        service::{
            Executor,
            test_utils::{set, stored},
        },
        utils::now_ms,
    };

    fn mset(items: &[&str]) -> Command {
        Command::Mset(items.iter().map(|s| s.to_string()).collect())
    }

    #[tokio::test]
    async fn sets_all_pairs_and_replies_ok() {
        let mut executor = Executor::default();
        let reply = executor
            .process_command_resp(mset(&["a", "1", "b", "2"]))
            .await
            .unwrap();

        assert_matches!(reply, Literal::String(s) if s == "OK");
        assert_eq!(stored(&executor, "a").await, Some(("1".to_string(), None)));
        assert_eq!(stored(&executor, "b").await, Some(("2".to_string(), None)));
    }

    #[tokio::test]
    async fn overwrites_existing_keys_and_clears_ttl() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set(
                "a",
                "old",
                None,
                Some(crate::command::SetArgumentExpiryRule::ExpiresAtMs(
                    now_ms() + 60_000,
                )),
                false,
            ))
            .await
            .unwrap();
        executor
            .process_command_resp(mset(&["a", "new"]))
            .await
            .unwrap();

        assert_eq!(
            stored(&executor, "a").await,
            Some(("new".to_string(), None))
        );
    }

    #[tokio::test]
    async fn last_value_wins_for_duplicate_keys() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(mset(&["a", "1", "a", "2"]))
            .await
            .unwrap();

        assert_eq!(stored(&executor, "a").await, Some(("2".to_string(), None)));
    }

    #[tokio::test]
    async fn odd_number_of_arguments_errors_and_sets_nothing() {
        let mut executor = Executor::default();
        let result = executor
            .process_command_resp(mset(&["a", "1", "b"]))
            .await;

        assert!(result.is_err());
        assert_eq!(stored(&executor, "a").await, None);
    }

    #[tokio::test]
    async fn no_arguments_errors() {
        let mut executor = Executor::default();
        assert!(executor.process_command_resp(mset(&[])).await.is_err());
    }
}
