# Slabcache

Slabcache is a simple LRU cache with slab allocation.
It wraps [slab-alloc](https://github.com/tokio-rs/slab)
with LRU eviction policy and maintains some simple statistics
for bare-bones observability.

Slab allocators allocate memory in fixed size chunks ahead of time.
Thus this library might be useful if you need to cache a fixed number of objects of the same type without incurring allocations at runtime, 
i.e. in performance critical scenarios where allocation has too much overhead. The metadata storage is 
pre-allocated as well, and thus all allocations happens at cache initialization. If the cache is ever at capacity,
it will evict the LRU element.



## Features

- Least Recently Used (LRU) cache eviction policy.
- Slab allocation for efficient memory usage.
- Statistics tracking for cache hits, misses, and current size.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
slabcache = "0.1.0"
```

## Usage
```rust
use slabcache::Cache;

fn main() {
    let mut cache = Cache::new(2);

    cache.insert("key1", "value1");
    cache.insert("key2", "value2");

    assert_eq!(cache.get("key1"), Some(&"value1"));
    assert_eq!(cache.get("key2"), Some(&"value2"));
    assert_eq!(cache.get("key3"), None);

    cache.insert("key3", "value3");

    // "key1" should be evicted because the cache capacity is 2
    assert_eq!(cache.get("key1"), None);
    assert_eq!(cache.get("key2"), Some(&"value2"));
    assert_eq!(cache.get("key3"), Some(&"value3"));
}
```