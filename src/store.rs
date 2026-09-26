use std::collections::HashMap;

use anyhow::Result;

pub trait Store {
    fn put(&mut self, key: String, value: String) -> Result<()>;
    fn get(&self, key: &String) -> Result<Option<String>>;
}

pub struct HashMapStore {
    broker: HashMap<String, String>, //TODO: need to be a mutex
}

impl HashMapStore {
    pub fn new() -> Self {
        HashMapStore {
            broker: HashMap::new(),
        }
    }
}

impl Store for HashMapStore {
    fn put(&mut self, key: String, value: String) -> Result<()> {
        self.broker.insert(key, value);
        Ok(())
    }

    fn get(&self, key: &String) -> Result<Option<String>> {
        Ok(self.broker.get(key).cloned())
    }
}
