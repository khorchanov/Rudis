use crate::{protocol::Literal, store::Store};

pub(super) async fn execute(store: &mut impl Store, keys: Vec<String>) -> anyhow::Result<Literal> {
    Ok(Literal::Integer(store.delete_many(keys).await? as i64))
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
    };

    #[tokio::test]
    async fn del_many_returns_number_of_deleted_keys() {
        let mut executor = Executor::default();
        for key in ["a", "b", "c"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }

        let keys = ["a", "b", "missing", "c"].map(String::from).to_vec();
        let reply = executor
            .process_command_resp(Command::Del(keys))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(3));
        for key in ["a", "b", "c"] {
            assert_eq!(stored(&executor, key).await, None);
        }
    }

    #[tokio::test]
    async fn del_many_returns_zero_when_no_key_exists() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("untouched", "v", None, None, false))
            .await
            .unwrap();

        let keys = ["x", "y", "z"].map(String::from).to_vec();
        let reply = executor
            .process_command_resp(Command::Del(keys))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Integer(0));
        assert_eq!(
            stored(&executor, "untouched").await,
            Some(("v".to_string(), None))
        );
    }
}
