use std::sync::Arc;

use rocksdb::DB;

pub trait KVStore {
    fn init(file_path: &str) -> Self;
    
    fn save(&self, key: &str, value: &str) -> bool;
    
    fn find(&self, key: &str) -> Option<String>;

    fn delete(&self, key: &str) -> bool;
}

#[derive(Clone)]
pub struct RocksDB {
    db: Arc<DB>,
}

impl KVStore for RocksDB {

    fn init(file_path: &str) -> Self {
        RocksDB { db: Arc::new(DB::open_default(file_path).unwrap()) }
    }

    fn save(&self, key: &str, value: &str) -> bool {
        self.db.put(key.as_bytes(), value.as_bytes()).is_ok()
    }

    fn find(&self, key: &str) -> Option<String> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let result = String::from_utf8(value).unwrap();
                // println!("Finding '{}' returns '{}'", key, result);
                Some(result)
            },
            Ok(None) => {
                // println!("Finding '{}' returns None", key);
                None
            },
            Err(e) => {
                // println!("Error retrieving value for {}: {}", key, e);
                None
            }
        }
    }

    fn delete(&self, key: &str) -> bool {
        self.db.delete(key.as_bytes()).is_ok()
    }
}
