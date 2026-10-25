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
            Literal::BulkString(value)
        } else {
            Literal::NullBulkString
        },
    )
}
