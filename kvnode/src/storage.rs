use std::collections::HashMap;

use tracing::{info, instrument};

#[derive(Default)]
pub struct KVStore {
    data: HashMap<String, String>,
}

pub trait StorageInterface {
    type K;
    type V;
    fn get(&self, key: &Self::K) -> Option<Self::V>;
    fn put(&mut self, key: &Self::K, value: &Self::V);
    fn delete(&mut self, key: &Self::K);
}

impl StorageInterface for KVStore {
    type K = String;

    type V = String;

    #[instrument(skip(self))]
    fn get(&self, key: &Self::K) -> Option<Self::V> {
        info!("GET entry");
        self.data.get(key).cloned()
    }

    #[instrument(skip(self))]
    fn put(&mut self, key: &Self::K, value: &Self::V) {
        self.data.insert(key.clone(), value.clone());
        info!("PUT entry");
    }

    #[instrument(skip(self))]
    fn delete(&mut self, key: &Self::K) {
        self.data.remove(key);
        info!("DELETE entry");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn can_put_and_get(xs: String, ys: String) -> bool {
        let mut store = KVStore::default();
        store.put(&xs, &ys);
        store.get(&xs) == Some(ys)
    }

    #[quickcheck]
    fn can_put_and_get_and_del_and_not_get(xs: String, ys: String) -> bool {
        let mut store = KVStore::default();
        store.put(&xs, &ys);
        store.delete(&xs);
        store.get(&xs).is_none()
    }

    #[quickcheck]
    fn put_overwrites(xs: String, ys: String, zs: String) -> bool {
        let mut store = KVStore::default();
        store.put(&xs, &ys);
        assert_eq!(store.get(&xs), Some(ys));
        store.put(&xs, &zs);
        store.get(&xs) == Some(zs)
    }
}
