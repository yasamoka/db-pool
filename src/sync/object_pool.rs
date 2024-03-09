// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::ops::{Deref, DerefMut};

pub type Stack<T> = Vec<T>;
type Init<T> = Box<dyn Fn() -> T + Send + Sync + 'static>;
type Reset<T> = Box<dyn Fn(&mut T) + Send + Sync + 'static>;

pub struct ObjectPool<T> {
    objects: Mutex<Stack<T>>,
    init: Init<T>,
    reset: Reset<T>,
}

impl<T> ObjectPool<T> {
    #[inline]
    pub fn new(
        init: impl Fn() -> T + Send + Sync + 'static,
        reset: impl Fn(&mut T) + Send + Sync + 'static,
    ) -> ObjectPool<T> {
        ObjectPool {
            objects: Mutex::new(Vec::new()),
            init: Box::new(init),
            reset: Box::new(reset),
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
    pub fn pull(&self) -> Reusable<T> {
        self.objects.lock().pop().map_or_else(
            || Reusable::new(self, (self.init)()),
            |data| Reusable::new(self, data),
        )
    }

    #[inline]
    pub fn attach(&self, mut t: T) {
        (self.reset)(&mut t);
        self.objects.lock().push(t);
    }
}

pub struct Reusable<'a, T> {
    pool: &'a ObjectPool<T>,
    data: Option<T>,
}

impl<'a, T> Reusable<'a, T> {
    #[inline]
    pub fn new(pool: &'a ObjectPool<T>, t: T) -> Self {
        Self {
            pool,
            data: Some(t),
        }
    }
}

const DATA_MUST_CONTAIN_SOME: &str = "data must always contain a [Some] value";

impl<'a, T> Deref for Reusable<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.as_ref().expect(DATA_MUST_CONTAIN_SOME)
    }
}

impl<'a, T> DerefMut for Reusable<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().expect(DATA_MUST_CONTAIN_SOME)
    }
}

impl<'a, T> Drop for Reusable<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.pool
            .attach(self.data.take().expect(DATA_MUST_CONTAIN_SOME));
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectPool;
    use std::mem::drop;

    #[test]
    fn len() {
        {
            let pool = ObjectPool::<Vec<u8>>::new(Vec::new, |_| {});

            let object1 = pool.pull();
            drop(object1);
            let object2 = pool.pull();
            drop(object2);

            assert_eq!(pool.len(), 1);
        }

        {
            let pool = ObjectPool::<Vec<u8>>::new(Vec::new, |_| {});

            let object1 = pool.pull();
            let object2 = pool.pull();

            drop(object1);
            drop(object2);
            assert_eq!(pool.len(), 2);
        }
    }

    #[test]
    fn e2e() {
        let pool = ObjectPool::new(Vec::new, |_| {});
        let mut objects = Vec::new();

        for i in 0..10 {
            let mut object = pool.pull();
            object.push(i);
            objects.push(object);
        }

        drop(objects);

        for i in (0..10).rev() {
            let mut object = pool.objects.lock().pop().expect("pool must have objects");
            assert_eq!(object.pop(), Some(i));
        }
    }

    #[test]
    fn reset() {
        let pool = ObjectPool::new(Vec::new, Vec::clear);

        let mut object = pool.pull();
        object.push(1);
        drop(object);
        let object = pool.pull();
        assert_eq!(object.len(), 0);
    }

    #[test]
    fn no_reset() {
        let pool = ObjectPool::new(Vec::new, |_| {});

        let mut object = pool.pull();
        object.push(1);
        drop(object);
        let object = pool.pull();
        assert_eq!(object.len(), 1);
    }
}
