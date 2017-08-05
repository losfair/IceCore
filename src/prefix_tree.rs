use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone, Debug)]
pub struct PrefixTree<'a, K: Eq + Hash + Clone, V> {
    root: Rc<RefCell<PrefixTreeNode<'a, K, V>>>,
}

#[derive(Clone, Debug)]
pub struct PrefixTreeNode<'a, K: Eq + Hash + Clone, V> {
    data: Option<V>,
    children: HashMap<K, Rc<RefCell<PrefixTreeNode<'a, K, V>>>>,
    default_child: Option<Rc<RefCell<PrefixTreeNode<'a, K, V>>>>
}

impl<'a, K, V> PrefixTree<'a, K, V> where K: Eq + Hash + Clone {
    pub fn new() -> PrefixTree<'a, K, V> {
        PrefixTree {
            root: Rc::new(RefCell::new(PrefixTreeNode::new()))
        }
    }

    pub fn insert<S, R>(&mut self, seq: Vec<Option<S>>, value: R) where S: AsRef<K>, R: Into<V> {
        let mut current_rc = self.root.clone();
        for i in 0..seq.len() {
            let ref item = seq[i];
            if item.is_none() {
                let next = match current_rc.borrow().default_child {
                    Some(ref v) => Some(v.clone()),
                    None => None
                };
                if next.is_none() {
                    current_rc = {
                        let dc = Rc::new(RefCell::new(PrefixTreeNode::new()));
                        let mut current = current_rc.borrow_mut();
                        current.default_child = Some(dc.clone());
                        dc
                    };
                } else {
                    current_rc = next.unwrap();
                }
            } else {
                let item = item.as_ref().unwrap();
                let item = item.as_ref();
                let next = match current_rc.borrow().children.get(&item) {
                    Some(v) => Some(v.clone()),
                    None => None
                };

                if next.is_none() {
                    current_rc = {
                        let mut current = current_rc.borrow_mut();
                        current.children.insert(item.clone(), Rc::new(RefCell::new(PrefixTreeNode::new())));
                        current.children.get(&item).unwrap().clone()
                    };
                } else {
                    current_rc = next.unwrap();
                }
            }
        }
        current_rc.borrow_mut().data = Some(value.into());
    }

    pub fn get<S>(&self, seq: Vec<S>) -> Option<Rc<RefCell<PrefixTreeNode<'a, K, V>>>> where S: AsRef<K> {
        let mut current_rc = self.root.clone();
        for i in 0..seq.len() {
            let item = seq[i].as_ref();
            let next;

            {
                let current = current_rc.borrow();
                next = match current.children.get(&item) {
                    Some(v) => v.clone(),
                    None => match current.default_child {
                        Some(ref v) => v.clone(),
                        None => return None
                    }
                };
            }

            current_rc = next;
        }
        Some(current_rc)
    }
}

impl<'a, K, V> PrefixTreeNode<'a, K, V> where K: Eq + Hash + Clone {
    pub fn new() -> PrefixTreeNode<'a, K, V> {
        PrefixTreeNode {
            data: None,
            children: HashMap::new(),
            default_child: None
        }
    }
}
