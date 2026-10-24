use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{SystemClock, service::Clock};

const EXPIRATION_DEPLAY: Duration = Duration::from_mins(5);

pub trait Store {
    fn put(
        &mut self,
        key: String,
        value: Entry,
    ) -> impl std::future::Future<Output = Result<Option<Entry>>>;
    fn get(&self, key: &str) -> impl std::future::Future<Output = Result<Option<Entry>>>;
    fn delete(&mut self, key: &str) -> impl std::future::Future<Output = Result<Option<Entry>>>;
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub value: String,
    pub expires_at: Instant,
}

impl Entry {
    pub fn expiring(value: String) -> Self {
        Entry {
            value,
            expires_at: Instant::now() + EXPIRATION_DEPLAY,
        }
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
            Some(entry) if self.clock.now() > entry.expires_at => {
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
}

#[cfg(test)]
mod test {
    use std::{
        assert_matches,
        cell::RefCell,
        sync::Arc,
        time::{Duration, Instant},
    };

    use crate::{
        HashMapStore,
        service::Clock,
        store::{Entry, Store},
    };

    #[derive(Clone)]
    pub struct TestClock {
        simulated_time: Arc<RefCell<Instant>>,
    }

    impl Default for TestClock {
        fn default() -> Self {
            Self {
                simulated_time: Arc::new(RefCell::new(Instant::now())),
            }
        }
    }

    impl TestClock {
        fn advance_time_by(&self, duration: Duration) {
            let old_value = self.simulated_time.borrow().clone();
            *self.simulated_time.borrow_mut() = old_value + duration;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            *self.simulated_time.borrow()
        }
    }

    #[tokio::test]
    async fn should_delete_on_expire() {
        let mut store = HashMapStore {
            clock: TestClock::default(),
            broker: Default::default(),
        };
        let (key, entry) = ("key".to_string(), Entry::expiring("value".to_string()));
        let _ = store.put(key.clone(), entry.clone()).await;
        let found = store.get(&key).await.unwrap().unwrap().value;
        assert_eq!(found, entry.value.clone());

        store.clock.advance_time_by(Duration::from_hours(1));

        let found = store.get(&key).await.unwrap();
        assert_matches!(found, None);
    }
}
