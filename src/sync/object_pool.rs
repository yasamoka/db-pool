// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::mem::{forget, ManuallyDrop};
use std::ops::{Deref, DerefMut};

pub type Stack<T> = Vec<T>;

pub struct ObjectPool<T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    objects: Mutex<Stack<T>>,
    init: I,
    reset: R,
}

impl<T, I, R> ObjectPool<T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    #[inline]
    pub fn new(init: I, reset: R) -> ObjectPool<T, I, R> {
        ObjectPool {
            objects: Mutex::new(Vec::new()),
            init,
            reset,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.objects.lock().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.objects.lock().is_empty()
    }

    #[inline]
    pub fn pull(&self) -> Reusable<T, I, R> {
        self.objects
            .lock()
            .pop()
            .map(|data| Reusable::new(self, data))
            .unwrap_or_else(|| Reusable::new(self, (self.init)()))
    }

    #[inline]
    pub fn attach(&self, mut t: T) {
        (self.reset)(&mut t);
        self.objects.lock().push(t)
    }
}

pub struct Reusable<'a, T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    pool: &'a ObjectPool<T, I, R>,
    data: ManuallyDrop<T>,
}

impl<'a, T, I, R> Reusable<'a, T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    #[inline]
    pub fn new(pool: &'a ObjectPool<T, I, R>, t: T) -> Self {
        Self {
            pool,
            data: ManuallyDrop::new(t),
        }
    }

    #[inline]
    pub fn detach(mut self) -> (&'a ObjectPool<T, I, R>, T) {
        let ret = unsafe { (self.pool, self.take()) };
        forget(self);
        ret
    }

    unsafe fn take(&mut self) -> T {
        ManuallyDrop::take(&mut self.data)
    }
}

impl<'a, T, I, R> Deref for Reusable<'a, T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T, I, R> DerefMut for Reusable<'a, T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T, I, R> Drop for Reusable<'a, T, I, R>
where
    I: Fn() -> T,
    R: Fn(&mut T),
{
    #[inline]
    fn drop(&mut self) {
        unsafe { self.pool.attach(self.take()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectPool, Reusable};
    use std::mem::drop;

    #[test]
    fn detach() {
        let pool = ObjectPool::new(|| Vec::new(), |_| {});
        let (pool, mut object) = pool.pull().detach();
        object.push(1);
        Reusable::new(&pool, object);
        assert_eq!(pool.pull()[0], 1);
    }

    #[test]
    fn detach_then_attach() {
        let pool = ObjectPool::new(|| Vec::new(), |_| {});
        let (pool, mut object) = pool.pull().detach();
        object.push(1);
        pool.attach(object);
        assert_eq!(pool.pull()[0], 1);
    }

    #[test]
    fn len() {
        {
            let pool = ObjectPool::<Vec<u8>, _, _>::new(|| Vec::new(), |_| {});

            let object1 = pool.pull();
            drop(object1);
            let object2 = pool.pull();
            drop(object2);

            assert_eq!(pool.len(), 1);
        }

        {
            let pool = ObjectPool::<Vec<u8>, _, _>::new(|| Vec::new(), |_| {});

            let object1 = pool.pull();
            let object2 = pool.pull();

            drop(object1);
            drop(object2);
            assert_eq!(pool.len(), 2);
        }
    }

    #[test]
    fn e2e() {
        let pool = ObjectPool::new(|| Vec::new(), |_| {});
        let mut objects = Vec::new();

        for i in 0..10 {
            let mut object = pool.pull();
            object.push(i);
            objects.push(object);
        }

        drop(objects);

        for i in (10..0).rev() {
            let mut object = pool.objects.lock().pop().unwrap();
            assert_eq!(object.pop(), Some(i));
        }
    }

    #[test]
    fn reset() {
        let pool = ObjectPool::new(|| Vec::new(), |v| v.clear());

        let mut object = pool.pull();
        object.push(1);
        drop(object);
        let object = pool.pull();
        assert_eq!(object.len(), 0);
    }

    #[test]
    fn no_reset() {
        let pool = ObjectPool::new(|| Vec::new(), |_| {});

        let mut object = pool.pull();
        object.push(1);
        drop(object);
        let object = pool.pull();
        assert_eq!(object.len(), 1);
    }
}
