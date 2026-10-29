use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(store: &mut impl Store, key: &str) -> anyhow::Result<Literal> {
    Ok(
        if let Some(Entry {
            value,
            expires_at_ms: Some(_),
        }) = store.get(key).await?
        {
            store
                .put(key.to_string(), Entry::never_expires(value))
                .await?;
            Literal::Integer(1)
        } else {
            Literal::Integer(0)
        },
    )
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::execute;
    use crate::{
        protocol::Literal,
        store::{Entry, HashMapStore, Store},
        utils::now_ms,
    };

    #[tokio::test]
    async fn should_remove_expiry_and_return_one() {
        let mut store = HashMapStore::default();
        let entry = Entry::expires_at_millis("value".to_string(), now_ms() + 60_000);
        store.put("k".to_string(), entry).await.unwrap();

        let result = execute(&mut store, "k").await.unwrap();

        assert_matches!(result, Literal::Integer(1));
        let found = store.get("k").await.unwrap().unwrap();
        assert_eq!(found.value, "value");
        assert_eq!(found.expires_at_ms, None);
    }

    #[tokio::test]
    async fn should_return_zero_when_key_has_no_expiry() {
        let mut store = HashMapStore::default();
        store
            .put("k".to_string(), Entry::never_expires("value".to_string()))
            .await
            .unwrap();

        let result = execute(&mut store, "k").await.unwrap();

        assert_matches!(result, Literal::Integer(0));
        assert_eq!(store.get("k").await.unwrap().unwrap().value, "value");
    }

    #[tokio::test]
    async fn should_return_zero_when_key_is_missing() {
        let mut store = HashMapStore::default();

        let result = execute(&mut store, "missing").await.unwrap();

        assert_matches!(result, Literal::Integer(0));
        assert!(!store.exists("missing").await.unwrap());
    }

    #[tokio::test]
    async fn should_return_zero_when_key_already_expired() {
        let mut store = HashMapStore::default();
        let entry = Entry::expires_at_millis("value".to_string(), now_ms().saturating_sub(1_000));
        store.put("k".to_string(), entry).await.unwrap();

        let result = execute(&mut store, "k").await.unwrap();

        assert_matches!(result, Literal::Integer(0));
        assert!(store.get("k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_return_zero_on_second_call() {
        let mut store = HashMapStore::default();
        let entry = Entry::expires_at_millis("value".to_string(), now_ms() + 60_000);
        store.put("k".to_string(), entry).await.unwrap();

        assert_matches!(execute(&mut store, "k").await.unwrap(), Literal::Integer(1));
        assert_matches!(execute(&mut store, "k").await.unwrap(), Literal::Integer(0));
    }
}
