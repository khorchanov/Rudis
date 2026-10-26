use crate::{
    HashMapStore,
    command::Command,
    protocol::Literal,
    store::{Store, SystemClock},
};

mod del;
mod echo;
mod exists;
mod expire;
mod get;
mod incr;
mod incr_by;
mod ping;
mod set;

pub struct Executor<S: Store> {
    pub store: S,
}

impl Default for Executor<HashMapStore<SystemClock>> {
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}

impl<S: Store> Executor<S> {
    pub async fn process_command_resp(&mut self, command: Command) -> anyhow::Result<Literal> {
        match command {
            Command::Ping(message) => Ok(ping::execute(message)),
            Command::Echo(message) => Ok(echo::execute(message)),
            Command::Set(command) => set::execute(&mut self.store, command).await,
            Command::Get(key) => get::execute(&self.store, &key).await,
            Command::Del(keys) => del::execute(&mut self.store, keys).await,
            Command::Exists(keys) => exists::execute(&self.store, keys).await,
            Command::Incr(key) => incr::execute(&mut self.store, &key).await,
            Command::IncrBy(key, by) => incr_by::execute(&mut self.store, &key, by).await,
            Command::Expire(key, duration) => {
                expire::execute(&mut self.store, &key, duration).await
            }
        }
    }
}

#[cfg(test)]
mod test_utils {
    use crate::{
        command::{Command, SetArgumentExpiryRule, SetArgumentRule, SetCommand},
        service::Executor,
        store::Store,
    };

    pub(super) fn set(
        key: &str,
        value: &str,
        set_rule: Option<SetArgumentRule>,
        expiry_rule: Option<SetArgumentExpiryRule>,
        return_old_value: bool,
    ) -> Command {
        Command::Set(SetCommand {
            key: key.to_string(),
            value: value.to_string(),
            return_old_value,
            set_rule,
            expiry_rule,
        })
    }

    pub(super) async fn stored(
        executor: &Executor<impl Store>,
        key: &str,
    ) -> Option<(String, Option<u64>)> {
        executor
            .store
            .get(key)
            .await
            .unwrap()
            .map(|entry| (entry.value, entry.expires_at_ms))
    }
}
