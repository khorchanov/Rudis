use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;

const EXPIRATION_DEPLAY: Duration = Duration::from_mins(5);

pub trait Store {
    fn put(&mut self, key: String, value: Entry) -> impl std::future::Future<Output = Result<()>>;
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

pub struct HashMapStore {
    broker: Arc<Mutex<HashMap<String, Entry>>>, //TODO: is it better to use a lock free actor pattern ?
}

impl Default for HashMapStore {
    fn default() -> Self {
        HashMapStore {
            broker: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Store for HashMapStore {
    async fn put(&mut self, key: String, value: Entry) -> Result<()> {
        let mut guard = self.broker.lock().expect("Store mutex poisoned");
        guard.insert(key, value);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Entry>> {
        let guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(guard.get(key).cloned()) //TODO; can't we not lock ? //TODO: can't we not clone ?
    }

    async fn delete(&mut self, key: &str) -> Result<Option<Entry>> {
        let mut guard = self.broker.lock().expect("Store mutex poisoned");
        Ok(guard.remove(key))
    }
}
