use std::collections::{HashMap, VecDeque};
use slab::Slab;

use chrono::Utc;
use crate::statistics::Statistics;


/// The metadata associated with each element in the cache
pub struct Metadata<K> {
    /// The last time the element was accessed as a UTC UNIX timestamp in us
    last_accessed: i64,
    /// The number of times the element has been accessed
    frequency: usize,
    /// The number of cache hits for the element
    hits: usize,
    /// The user-provided key for the element
    user_key: K,
}
pub struct CacheIter<'a, K, V> {
    usage: std::collections::vec_deque::Iter<'a, usize>,
    cache: &'a Cache<K, V>,
}

pub struct CacheIterFrequency<'a, K, V> {
    keys: std::vec::IntoIter<usize>,
    cache: &'a Cache<K, V>,
}

impl<'a, K, V> Iterator for CacheIterFrequency<'a, K, V> {
    type Item = (&'a K, &'a V, &'a Metadata<K>);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.next()?;
        let value = self.cache.slab.get(key)?;
        let metadata = self.cache.key_meta.get(&key)?;
        Some((&metadata.user_key, value, metadata))
    }
}

pub enum SortOrder {
    Ascending,
    Descending,
}


impl<'a, K, V> Iterator for CacheIter<'a, K, V> {
    type Item = (&'a K, &'a V, &'a Metadata<K>);

    fn next(&mut self) -> Option<Self::Item> {
        self.usage.next().and_then(|key| {
            let value = self.cache.slab.get(*key)?;
            let metadata = self.cache.key_meta.get(key)?;
            Some((&metadata.user_key, value, metadata))
        })
    }
}
/// An efficient LRU in-memory cache based on a slab allocator.
///
/// # Examples
///.```rust
///
/// use slabcache::Cache;
///let mut cache = Cache::new(3);
///
/// cache.insert("foo", "bar");
/// cache.insert("baz", "bar");
/// cache.insert("foobar", "barbaz");
///
/// // Access elements
/// let _ = cache.get("foo");
/// let _ = cache.get("baz");
///
/// // Insert another element to force eviction of the LRU element
///
/// cache.insert("key", "value");
///
/// assert_eq!(cache.get("foo"), Some(&"bar"));
/// assert_eq!(cache.get("baz"), Some(&"bar"));
/// assert_eq!(cache.get("foobar"), None));
/// assert_eq!(cache.get("key"), Some(&"value"));
///
/// // Iterate over the cache elements by access frequency
/// for (key, value, metadata) in cache.iter_frequency(SortOrder::Ascending) {
///     println!("Key: {}, Value: {}, Frequency: {}", key, value, metadata.frequency);
/// }```
pub struct Cache<K, V> {
    /// The slab allocator used as the storage engine for the cache
    slab: Slab<V>,
    /// A map from the index of an element in the slab to its metadata
    key_meta: HashMap<usize, Metadata<K>>,
    /// A map from the user-provided key to the index of the element in the slab
    key_map: HashMap<K, usize>,
    /// A list of indices of elements in the slab to enforce the LRU policy
    usage: VecDeque<usize>,
    /// A map from the index of an element in the slab to its position in the usage list to provide O(1) access
    usage_map: HashMap<usize, usize>,
    /// The maximum number of elements that the cache can hold
    capacity: usize,
    /// Statistics about the cache
    statistics: Statistics,
}
impl<K: std::hash::Hash + Eq + Clone, V> Cache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Cache {
            slab: Slab::with_capacity(capacity),
            key_meta: HashMap::with_capacity(capacity),
            key_map: HashMap::with_capacity (capacity),
            usage: VecDeque::with_capacity(capacity),
            usage_map: HashMap::with_capacity(capacity),
            statistics: Statistics::new(),
            capacity,
        }
    }

    /// Insert a value into the cache
    pub fn insert(&mut self, key: K, value: V) -> K {
        let index= self.slab.insert(value);
        self.key_meta.insert(
            index,
            Metadata {
                last_accessed: 0,
                frequency: 0,
                hits: 0,
                user_key: key.clone(),
            },
        );
        self.key_map.insert(key.clone(), index);
        self.usage.push_back(index);
        if self.usage.len() > self.capacity {
            if let Some(key) = self.usage.pop_front() {
                let metadata = self.key_meta.get(&key).unwrap();
                self.slab.remove(key);
                self.usage_map.remove(&key);
                self.key_map.remove(&metadata.user_key);
                self.key_meta.remove(&key);
            }
        }
        self.statistics.update_size(self.slab.len());
        key
    }


    /// Get a value from the cache and update its access time and frequency
    pub fn get(&mut self, key: K) -> Option<&V> {
        match self.key_map.get(&key) {
            Some(&usize_key) => {
                if let Some(meta) = self.key_meta.get_mut(&usize_key) {
                    meta.last_accessed = Utc::now().timestamp_micros();
                    meta.frequency += 1;
                    meta.hits += 1;
                    self.statistics.hit();
                }
                if let Some(&position) = self.usage_map.get(&usize_key) {
                    let k = self.usage.remove(position)?;
                    self.usage.push_back(k);
                    self.usage_map.insert(usize_key, self.usage.len() - 1);
                }
                self.slab.get(usize_key)
            }
            None => {
                self.statistics.miss();
                None
            }
        }
    }


    /// Return the least recently used element in the cache
    pub fn get_lru(&self) -> Option<&V> {
        let key = self.usage.front()?;
        self.slab.get(*key)
    }


    /// Remove all elements from the cache but preserve allocated memory
    pub fn flush(&mut self) {
        self.slab.clear();
        self.key_meta.clear();
        self.usage.clear();
        self.usage_map.clear();
        self.key_map.clear();
    }


    /// Returns an iterator over the cache in order of access frequency
    pub fn iter_frequency(&self, order: SortOrder) -> CacheIterFrequency<K, V> {
        let mut keys: Vec<usize> = self.key_meta.keys().cloned().collect();
        keys.sort_by_key(|k| self.key_meta.get(k).unwrap().frequency);
        if let SortOrder::Descending = order {
            keys.reverse();
        }
        CacheIterFrequency {
            keys: keys.into_iter(),
            cache: self,
        }
    }
}



#[cfg(test)]
#[test]
fn test_cache_basic() {
    let mut cache = Cache::new(10);
    let key = cache.insert("hello", "world");
    assert_eq!(cache.get(key), Some(&"world"));
}

#[test]
fn test_lru_eviction() {
    let mut cache = Cache::new(2);

    let key1 = cache.insert("key1", "value1");
    let key2 = cache.insert("key2", "value2");

    let _value = cache.get(key1);
    let _value = cache.get(key2);
    let _value  = cache.get(key2);

    // At this point, the cache is full. The next insert should evict the least recently used item (key1).
    let key3 = cache.insert("key3", "value3");

    // Check that the value associated with key1 has been evicted.
    assert_eq!(cache.get(key1), None);

    // Check that the values associated with key2 and key3 are still in the cache.
    assert_eq!(cache.get(key2), Some(&"value2"));
    assert_eq!(cache.get(key3), Some(&"value3"));
}

#[test]
fn test_get_lru_element() {
    let mut cache = Cache::new(2);
    let key1 = cache.insert("key1", "value1");
    let _key2 = cache.insert("key2", "value2");

    let _value = cache.get(key1);

    assert_eq!(cache.get_lru(), Some(&"value1"));
}

#[test]
fn test_frequency_iter() {
    let mut cache = Cache::new(3);

    let key1 = cache.insert("key1", "value1");
    let key2 = cache.insert("key2", "value2");
    let key3 = cache.insert("key3", "value3");

    let _ = cache.get(key1);
    let _ = cache.get(key1);
    let _ = cache.get(key1);
    let _ = cache.get(key2);
    let _ = cache.get(key2);
    let _ = cache.get(key3);

    let ascending_keys: Vec<&str> = cache.iter_frequency(SortOrder::Ascending).map(|(k, _, _)| k).cloned().collect();
    let descending_keys: Vec<&str> = cache.iter_frequency(SortOrder::Descending).map(|(k, _, _)| k).cloned().collect();
    assert_eq!(ascending_keys, vec!["key3", "key2", "key1"]);
    assert_eq!(descending_keys, vec!["key1", "key2", "key3"]);
}
#[test]
fn test_statistics() {
    let mut cache = Cache::new(3);

    cache.insert("key1", "value1");
    cache.insert("key2", "value2");
    cache.insert("key3", "value3");

    cache.get("key1");
    cache.get("key2");
    cache.get("key4"); // Miss

    assert_eq!(cache.statistics.get_hits(), 2);
    assert_eq!(cache.statistics.get_misses(), 1);
    assert_eq!(cache.statistics.get_current_size(), 3);
}
#[test]
fn test_metadata_fields() {
    let mut cache = Cache::new(3);

    let key1 = cache.insert("key1", "value1");
    let key2 = cache.insert("key2", "value2");

    cache.get(key1);
    cache.get(key1);
    cache.get(key2);
    // Force a miss
    cache.get("key4");

    let meta1 = cache.key_meta.get(&cache.key_map[&key1]).unwrap();
    let meta2 = cache.key_meta.get(&cache.key_map[&key2]).unwrap();

    assert!(meta1.last_accessed > 0);
    assert_eq!(meta1.frequency, 2);
    assert_eq!(meta1.hits, 2);

    assert!(meta2.last_accessed > 0);
    assert_eq!(meta2.frequency, 1);
    assert_eq!(meta2.hits, 1);
}

