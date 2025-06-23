// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::ops::{Deref, DerefMut};

type Stack<T> = Vec<T>;
type Init<T> = Box<dyn Fn() -> T + Send + Sync + 'static>;
type Reset<T> = Box<dyn Fn(&mut T) + Send + Sync + 'static>;

/// Object pool
pub struct ObjectPool<T> {
    objects: Mutex<Stack<T>>,
    init: Init<T>,
    reset: Reset<T>,
}

impl<T> ObjectPool<T> {
    pub(crate) fn new(
        init: impl Fn() -> T + Send + Sync + 'static,
        reset: impl Fn(&mut T) + Send + Sync + 'static,
    ) -> ObjectPool<T> {
        ObjectPool {
            objects: Mutex::new(Vec::new()),
            init: Box::new(init),
            reset: Box::new(reset),
        }
    }

    pub(crate) fn pull(&self) -> Reusable<T> {
        self.objects.lock().pop().map_or_else(
            || Reusable::new(self, (self.init)()),
            |mut data| {
                (self.reset)(&mut data);
                Reusable::new(self, data)
            },
        )
    }

    fn attach(&self, t: T) {
        self.objects.lock().push(t);
    }
}

/// Reusable object wrapper
pub struct Reusable<'a, T> {
    pool: &'a ObjectPool<T>,
    data: Option<T>,
}

impl<'a, T> Reusable<'a, T> {
    fn new(pool: &'a ObjectPool<T>, t: T) -> Self {
        Self {
            pool,
            data: Some(t),
        }
    }
}

const DATA_MUST_CONTAIN_SOME: &str = "data must always contain a [Some] value";

impl<T> Deref for Reusable<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().expect(DATA_MUST_CONTAIN_SOME)
    }
}

impl<T> DerefMut for Reusable<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().expect(DATA_MUST_CONTAIN_SOME)
    }
}

impl<T> Drop for Reusable<'_, T> {
    fn drop(&mut self) {
        self.pool
            .attach(self.data.take().expect(DATA_MUST_CONTAIN_SOME));
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectPool;
    use std::mem::drop;

    impl<T> ObjectPool<T> {
        fn len(&self) -> usize {
            self.objects.lock().len()
        }
    }

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
