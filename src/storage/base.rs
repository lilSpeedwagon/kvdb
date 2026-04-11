use crate::types;


pub trait KvStorage {
    fn set(&mut self, key: types::Key, value: types::Value) -> types::Result<()>;
    fn get(&mut self, key: types::Key) -> types::Result<Option<types::Value>>;
    fn remove(&mut self, key: types::Key) -> types::Result<bool>;
}
