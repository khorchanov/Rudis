use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use crate::utils::now_ms;

pub trait Clock {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock {}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        now_ms()
    }
}

pub trait Store {
    fn put(
        &mut self,
        key: String,
        value: Entry,
    ) -> impl std::future::Future<Output = Result<Option<Entry>>>;
    fn get(&self, key: &str) -> impl std::future::Future<Output = Result<Option<Entry>>>;
    fn exists(&self, key: &str) -> impl std::future::Future<Output = Result<bool>>;
    fn delete(&mut self, key: &str) -> impl std::future::Future<Output = Result<Option<Entry>>>;
    fn delete_many(&mut self, key: Vec<String>) -> impl std::future::Future<Output = Result<u64>>;
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub value: String,
    pub expires_at_ms: Option<u64>, //TODO: make this invisible outside of this module
}

impl Entry {
    pub fn never_expires(value: String) -> Self {
        Entry {
            value,
            expires_at_ms: None,
        }
    }

    pub fn expiring_at_millis(value: String, timestamp: u64) -> Self {
        Entry {
            value,
            expires_at_ms: Some(timestamp),
        }
    }

    pub fn may_expires_in_millis(value: String, timestamp: Option<u64>) -> Self {
        if let Some(ts) = timestamp {
            Self::expiring_at_millis(value, ts)
        } else {
            Self::never_expires(value)
        }
    }

    pub fn expired(&self, clock: &impl Clock) -> bool {
        self.expires_at_ms
            .map(|expires_at_ms| clock.now_ms() > expires_at_ms)
            .unwrap_or_default()
    }
}

pub struct HashMapStore<C: Clock> {
    clock: C,
    broker: Arc<Mutex<HashMap<String, Entry>>>, //TODO: is it better to use a lock free actor pattern ?
}

impl Default for HashMapStore<SystemClock> {
    fn default() -> Self {
        HashMapStore {
            clock: SystemClock {},
            broker: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<C: Clock> Store for HashMapStore<C> {
    async fn put(&mut self, key: String, value: Entry) -> Result<Option<Entry>> {
        let mut guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(guard.insert(key, value))
    }

    async fn get(&self, key: &str) -> Result<Option<Entry>> {
        let mut guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(match guard.get(key) {
            Some(entry) if entry.expired(&self.clock) => {
                guard.remove(key);
                None
            }
            Some(entry) => Some(entry.clone()),
            None => None,
        })
    }

    async fn delete(&mut self, key: &str) -> Result<Option<Entry>> {
        let mut guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(guard.remove(key))
    }

    async fn delete_many(&mut self, keys: Vec<String>) -> Result<u64> {
        let mut count = 0;
        for key in keys {
            if self.delete(&key).await?.is_some() {
                count += 1;
            };
        }
        Ok(count)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(guard
            .get(key)
            .map(|e| !e.expired(&self.clock))
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod test {
    use std::{assert_matches, cell::RefCell, sync::Arc};

    use crate::{
        HashMapStore,
        store::{Clock, Entry, Store},
        utils::now_ms,
    };

    #[derive(Clone)]
    pub struct TestClock {
        simulated_time: Arc<RefCell<u64>>,
    }

    impl Default for TestClock {
        fn default() -> Self {
            Self {
                simulated_time: Arc::new(RefCell::new(now_ms())),
            }
        }
    }

    impl TestClock {
        fn advance_time_by(&self, duration: u64) {
            let old_value = self.simulated_time.borrow().clone();
            *self.simulated_time.borrow_mut() = old_value + duration;
        }
    }

    impl Clock for TestClock {
        fn now_ms(&self) -> u64 {
            *self.simulated_time.borrow()
        }
    }

    #[tokio::test]
    async fn should_delete_on_expire() {
        let mut store = HashMapStore {
            clock: TestClock::default(),
            broker: Default::default(),
        };
        let (key, entry) = (
            "key".to_string(),
            Entry::expiring_at_millis("value".to_string(), now_ms() + 10 * 60 * 1000),
        );
        let _ = store.put(key.clone(), entry.clone()).await;
        let found = store.get(&key).await.unwrap().unwrap().value;
        assert_eq!(found, entry.value.clone());

        store.clock.advance_time_by(now_ms() + 30 * 60 * 1000);

        let found = store.get(&key).await.unwrap();
        assert_matches!(found, None);
    }
}
