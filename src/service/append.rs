use crate::{
    protocol::Literal,
    store::{Entry, Store},
};

pub(super) async fn execute(
    store: &mut impl Store,
    key: String,
    value: String,
) -> anyhow::Result<Literal> {
    Ok(if let Some(mut entry) = store.get(&key).await? {
        entry.value = format!("{}{}", entry.value, value);
        let len = entry.value.len();
        store.put(key.to_string(), entry).await?;
        Literal::Integer(len.try_into()?)
    } else {
        let entry = Entry::never_expires(value);
        let len = entry.value.len();
        store.put(key.to_string(), entry).await?;
        Literal::Integer(len.try_into()?)
    })
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
    async fn should_create_key_when_missing() {
        let mut store = HashMapStore::default();

        let result = execute(&mut store, "k".into(), "Hello".into())
            .await
            .unwrap();

        assert_matches!(result, Literal::Integer(5));
        let found = store.get("k").await.unwrap().unwrap();
        assert_eq!(found.value, "Hello");
        assert_eq!(found.expires_at_ms, None);
    }

    #[tokio::test]
    async fn should_append_to_existing_value() {
        let mut store = HashMapStore::default();
        execute(&mut store, "k".into(), "Hello".into())
            .await
            .unwrap();

        let result = execute(&mut store, "k".into(), " World".into())
            .await
            .unwrap();

        assert_matches!(result, Literal::Integer(11));
        assert_eq!(store.get("k").await.unwrap().unwrap().value, "Hello World");
    }

    #[tokio::test]
    async fn should_preserve_ttl() {
        let mut store = HashMapStore::default();
        let expires_at = now_ms() + 60_000;
        store
            .put(
                "k".to_string(),
                Entry::expires_at_millis("a".to_string(), expires_at),
            )
            .await
            .unwrap();

        execute(&mut store, "k".into(), "b".into()).await.unwrap();

        let found = store.get("k").await.unwrap().unwrap();
        assert_eq!(found.value, "ab");
        assert_eq!(found.expires_at_ms, Some(expires_at));
    }

    #[tokio::test]
    async fn should_treat_expired_key_as_missing() {
        let mut store = HashMapStore::default();
        store
            .put(
                "k".to_string(),
                Entry::expires_at_millis("old".to_string(), now_ms().saturating_sub(1_000)),
            )
            .await
            .unwrap();

        let result = execute(&mut store, "k".into(), "new".into()).await.unwrap();

        assert_matches!(result, Literal::Integer(3));
        let found = store.get("k").await.unwrap().unwrap();
        assert_eq!(found.value, "new");
        assert_eq!(found.expires_at_ms, None);
    }

    #[tokio::test]
    async fn should_append_empty_string() {
        let mut store = HashMapStore::default();
        execute(&mut store, "k".into(), "abc".into()).await.unwrap();

        let result = execute(&mut store, "k".into(), "".into()).await.unwrap();

        assert_matches!(result, Literal::Integer(3));
        assert_eq!(store.get("k").await.unwrap().unwrap().value, "abc");
    }

    #[tokio::test]
    async fn should_count_length_in_bytes() {
        let mut store = HashMapStore::default();

        let result = execute(&mut store, "k".into(), "é".into()).await.unwrap();

        assert_matches!(result, Literal::Integer(2));
    }
}
