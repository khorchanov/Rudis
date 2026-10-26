use std::time::Duration;

use crate::{protocol::Literal, store::Store, utils::now_ms};

pub(super) async fn execute(
    store: &mut impl Store,
    key: &str,
    duration: Option<Duration>,
) -> anyhow::Result<Literal> {
    Ok(match (store.get(key).await?, duration) {
        (None, None) | (None, Some(_)) => Literal::Integer(0),
        (Some(_), None) => {
            store.delete(key).await?;
            Literal::Integer(1)
        }
        (Some(mut entry), Some(duration)) => {
            entry.expires_at_ms = Some(now_ms() + duration.as_millis() as u64); //TODO: fix this
            store.put(key.to_owned(), entry).await?;
            Literal::Integer(1)
        }
    })
}

#[cfg(test)]
mod test {
    use std::{assert_matches, time::Duration};

    use crate::{
        command::Command,
        protocol::Literal,
        service::{
            Executor,
            test_utils::{set, stored},
        },
        utils::now_ms,
    };

    fn expire(key: &str, duration: Duration) -> Command {
        Command::Expire(key.to_string(), Some(duration))
    }

    #[tokio::test]
    async fn expire_sets_expiry_on_existing_key() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v", None, None, false))
            .await
            .unwrap();

        let before = now_ms();
        let reply = executor
            .process_command_resp(expire("k", Duration::from_secs(10)))
            .await
            .unwrap();
        let after = now_ms();

        assert_matches!(reply, Literal::Integer(1));
        let (value, expires_at) = stored(&executor, "k").await.unwrap();
        let expires_at = expires_at.expect("expiry should be set");
        assert_eq!(value, "v");
        assert!((before + 10_000..=after + 10_000).contains(&expires_at));
    }

    #[tokio::test]
    async fn pexpire_style_milliseconds_are_respected() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v", None, None, false))
            .await
            .unwrap();

        let before = now_ms();
        executor
            .process_command_resp(expire("k", Duration::from_millis(1500)))
            .await
            .unwrap();
        let after = now_ms();

        let (_, expires_at) = stored(&executor, "k").await.unwrap();
        let expires_at = expires_at.expect("expiry should be set");
        assert!((before + 1500..=after + 1500).contains(&expires_at));
    }

    #[tokio::test]
    async fn expire_missing_key_returns_zero() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(expire("missing", Duration::from_secs(10)))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(0));
        assert_eq!(stored(&executor, "missing").await, None);
    }

    #[tokio::test]
    async fn expire_overrides_previous_expiry() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v", None, None, false))
            .await
            .unwrap();
        executor
            .process_command_resp(expire("k", Duration::from_secs(1000)))
            .await
            .unwrap();

        let before = now_ms();
        executor
            .process_command_resp(expire("k", Duration::from_secs(1)))
            .await
            .unwrap();

        let (_, expires_at) = stored(&executor, "k").await.unwrap();
        assert!(expires_at.unwrap() < before + 5_000);
    }

    #[tokio::test]
    async fn expire_with_zero_duration_makes_key_disappear() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("k", "v", None, None, false))
            .await
            .unwrap();

        executor
            .process_command_resp(expire("k", Duration::ZERO))
            .await
            .unwrap();
        std::thread::sleep(Duration::from_millis(5));

        assert_eq!(stored(&executor, "k").await, None);
    }
}
