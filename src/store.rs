use std::{collections::HashMap, time::{Duration, Instant}};

use anyhow::Result;

pub trait Store {
    fn put(&mut self, key: String, value: Entry) -> Result<()>;
    fn get(&self, key: &str) -> Result<Option<Entry>>;
}

#[derive(Clone)]
pub struct Entry {
    pub value: String,
    pub expires_at: Option<Instant>
}

impl Entry {
    pub fn expiring(value : String) -> Self {
        Entry {
            value,
            expires_at: Some(Instant::now() + Duration::from_mins(5))
        }
    }
}

pub struct HashMapStore {
    broker: HashMap<String, Entry>, //TODO: need to be a mutex
}

impl HashMapStore {
    pub fn new() -> Self {
        HashMapStore {
            broker: HashMap::new(),
        }
    }
}

impl Store for HashMapStore {
    fn put(&mut self, key: String, value: Entry) -> Result<()> {
        self.broker.insert(key, value);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<Entry>> {
        Ok(self.broker.get(key).cloned())
    }
}
