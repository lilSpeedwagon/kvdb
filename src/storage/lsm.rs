use crate::storage::base;
use crate::types;


struct LsmStorage {}

impl base::KvStorage for LsmStorage {
    fn set(&mut self, key: types::Key, value: types::Value) -> types::Result<()> {
        todo!()
    }

    fn get(&mut self, key: types::Key) -> types::Result<Option<types::Value>> {
        todo!()
    }

    fn remove(&mut self, key: types::Key) -> types::Result<bool> {
        todo!()
    }
}
