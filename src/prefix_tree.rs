use std;
use std::hash::Hash;
use std::collections::HashMap;

pub struct PrefixTree<K, V> where K: Hash + Eq + Clone, V: Clone {
    root: Node<K, V>
}

struct Node<K, V> where K: Hash + Eq + Clone, V: Clone {
    value: Option<V>,
    children: HashMap<K, *mut Node<K, V>>
}

unsafe impl<K, V> Send for Node<K, V> where K: Hash + Eq + Clone, V: Clone {}
unsafe impl<K, V> Sync for Node<K, V> where K: Hash + Eq + Clone, V: Clone {}

impl<K, V> PrefixTree<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub fn new() -> PrefixTree<K, V> {
        PrefixTree {
            root: Node::new(None)
        }
    }

    pub fn insert(&mut self, seq: &[K], value: V) {
        let mut current: *mut Node<K, V> = &mut self.root;

        for item in seq {
            current = unsafe { (&mut *current) }.get_or_create_child(item);
        }

        unsafe { (&mut *current) }.value = Some(value);
    }

    pub fn find(&self, seq: &[K], default_key: Option<&K>) -> Option<V> {
        let mut current: *const Node<K, V> = &self.root;

        for item in seq {
            current = match unsafe { (&*current) }.get_child(item) {
                Some(v) => v,
                None => if default_key.is_some() {
                    match unsafe { (&*current) }.get_child(default_key.unwrap()) {
                        Some(v) => v,
                        None => return None
                    }
                } else {
                    return None
                }
            };
        }

        unsafe { (&*current) }.value.clone()
    }
}

impl<K, V> Node<K, V> where K: Hash + Eq + Clone, V: Clone {
    fn new(v: Option<V>) -> Node<K, V> {
        Node {
            value: v,
            children: HashMap::new()
        }
    }

    fn get_or_create_child(&mut self, key: &K) -> *mut Node<K, V> {
        let mut v = match self.children.get(key) {
            Some(v) => *v,
            None => std::ptr::null_mut()
        };
        if v.is_null() {
            v = self.add_child(key.clone(), Node::new(None));
        }
        v
    }

    fn get_child(&self, key: &K) -> Option<*mut Node<K, V>> {
        match self.children.get(key) {
            Some(v) => Some(*v),
            None => None
        }
    }

    fn add_child(&mut self, key: K, value: Node<K, V>) -> *mut Node<K, V> {
        self.remove_child(&key);

        let v_ref = Box::into_raw(Box::new(value));
        self.children.insert(key, v_ref);

        v_ref
    }

    fn remove_child(&mut self, key: &K) {
        match self.children.remove(key) {
            Some(v) => {
                unsafe {
                    Box::from_raw(v);
                }
            },
            None => {}
        }
    }
}

impl<K, V> Drop for Node<K, V> where K: Hash + Eq + Clone, V: Clone {
    fn drop(&mut self) {
        let keys: Vec<K> = self.children.iter().map(|(k, _)| k.clone()).collect();
        for k in keys {
            self.remove_child(&k);
        }
    }
}
