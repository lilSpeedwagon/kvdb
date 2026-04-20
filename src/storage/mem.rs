use crate::storage::base;
use crate::types;


pub struct MemStorage {
    storage: std::collections::HashMap<types::Key, types::Value>,
}

impl MemStorage {
    pub fn new() -> MemStorage {
        MemStorage { storage: std::collections::HashMap::new() }
    }
}

impl base::KvStorage for MemStorage {
    fn set(&mut self, key: types::Key, value: types::Value) -> types::Result<()> {
        self.storage.insert(key, value);
        Ok(())
    }
    
    fn get(&mut self, key: types::Key) -> types::Result<Option<types::Value>> {
        let val_opt = self.storage.get(&key);
        match val_opt {
            Some(val) => Ok(Some(val.clone())),
            None => Ok(None),
        }
    }
    
    fn remove(&mut self, key: types::Key) -> types::Result<bool> {
        let val_opt = self.storage.remove(&key);
        Ok(val_opt.is_some())
    }
}
