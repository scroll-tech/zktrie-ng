use super::KVDatabase;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

impl<Db: KVDatabase> KVDatabase for RwLock<Db> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.read().unwrap().contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.write().unwrap().put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.write().unwrap().or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.write().unwrap().or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.write().unwrap().put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.read().unwrap().get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.read().unwrap().is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, _gc_enabled: bool) {
        self.write().unwrap().set_gc_enabled(_gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.read().unwrap().gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.write().unwrap().remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.write().unwrap().retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.write().unwrap().extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for Mutex<Db> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.lock().unwrap().contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.lock().unwrap().put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.lock().unwrap().or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.lock().unwrap().or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.lock().unwrap().put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.lock().unwrap().get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.lock().unwrap().is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.lock().unwrap().set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.lock().unwrap().gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.lock().unwrap().remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.lock().unwrap().retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.lock().unwrap().extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for RefCell<Db> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.borrow().contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow().get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.borrow().is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.borrow_mut().set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.borrow().gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.borrow_mut().retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for Rc<RefCell<Db>> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.borrow().contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow().get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.borrow().is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.borrow_mut().set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.borrow().gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.borrow_mut().retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for Arc<RefCell<Db>> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.borrow().contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow_mut().put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.borrow().get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.borrow().is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.borrow_mut().set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.borrow().gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.borrow_mut().remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.borrow_mut().retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.borrow_mut().extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for Box<Db> {
    type Item = Db::Item;
    type Error = Db::Error;

    #[inline(always)]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        (**self).contains_key(k)
    }

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        (**self).put(k, v)
    }

    #[inline(always)]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        (**self).or_put(k, v)
    }

    #[inline(always)]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        (**self).or_put_with(k, default)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        (**self).put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        (**self).get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        (**self).is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        (**self).set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        (**self).gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        (**self).remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        (**self).retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        (**self).extend(other)
    }
}

impl<Db: KVDatabase> KVDatabase for &mut Db {
    type Item = Db::Item;

    type Error = Db::Error;

    #[inline(always)]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        (*self).put(k, v)
    }

    #[inline(always)]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        (*self).put_owned(k, v)
    }

    #[inline(always)]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        (&**self).get(k)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        (&**self).is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        (*self).set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        (&**self).gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        (*self).remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        (*self).retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        (*self).extend(other)
    }
}
