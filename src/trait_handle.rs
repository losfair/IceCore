use std::ops::{Deref, DerefMut};

pub struct TraitHandle<T> where T: ?Sized {
    inner: Box<T>
}

impl<T> Deref for TraitHandle<T> where T: ?Sized {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.inner
    }
}

impl<T> DerefMut for TraitHandle<T> where T: ?Sized {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.inner
    }
}

impl<T> From<Box<T>> for TraitHandle<T> where T: ?Sized {
    fn from(other: Box<T>) -> TraitHandle<T> {
        TraitHandle {
            inner: other
        }
    }
}

impl<T> From<T> for TraitHandle<T> {
    fn from(other: T) -> TraitHandle<T> {
        TraitHandle {
            inner: Box::new(other)
        }
    }
}

impl<T> Into<Box<T>> for TraitHandle<T> {
    fn into(self) -> Box<T> {
        self.inner
    }
}
