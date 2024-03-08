// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::future::Future;
use std::mem::{forget, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

pub type Stack<T> = Vec<T>;

type Init<T> =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static>;
type Reset<T> =
    Box<dyn Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static>;

pub struct AsyncObjectPool<T> {
    objects: Mutex<Stack<T>>,
    init: Init<T>,
    reset: Reset<T>,
}

impl<T> AsyncObjectPool<T> {
    #[inline]
    pub fn new(
        init: impl Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static,
        reset: impl Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + Sync + 'static,
    ) -> AsyncObjectPool<T> {
        AsyncObjectPool {
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
    pub async fn pull(&self) -> AsyncReusable<T> {
        let object = self.objects.lock().pop();
        let object = if let Some(object) = object {
            (self.reset)(object).await
        } else {
            (self.init)().await
        };
        AsyncReusable::new(self, object)
    }

    #[inline]
    pub fn attach(&self, t: T) {
        self.objects.lock().push(t);
    }
}

pub struct AsyncReusable<'a, T> {
    pool: &'a AsyncObjectPool<T>,
    data: ManuallyDrop<T>,
}

impl<'a, T> AsyncReusable<'a, T> {
    #[inline]
    pub fn new(pool: &'a AsyncObjectPool<T>, t: T) -> Self {
        Self {
            pool,
            data: ManuallyDrop::new(t),
        }
    }

    #[inline]
    pub fn detach(mut self) -> (&'a AsyncObjectPool<T>, T) {
        let ret = unsafe { (self.pool, self.take()) };
        forget(self);
        ret
    }

    unsafe fn take(&mut self) -> T {
        ManuallyDrop::take(&mut self.data)
    }
}

impl<'a, T> Deref for AsyncReusable<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for AsyncReusable<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T> Drop for AsyncReusable<'a, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.pool.attach(self.take()) }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::{AsyncObjectPool, Reusable};
//     use std::mem::drop;

//     #[test]
//     fn detach() {
//         let pool = AsyncObjectPool::new(|| Vec::new(), |_| {});
//         let (pool, mut object) = pool.pull().detach();
//         object.push(1);
//         Reusable::new(&pool, object);
//         assert_eq!(pool.pull()[0], 1);
//     }

//     #[test]
//     fn detach_then_attach() {
//         let pool = AsyncObjectPool::new(|| Vec::new(), |_| {});
//         let (pool, mut object) = pool.pull().detach();
//         object.push(1);
//         pool.attach(object);
//         assert_eq!(pool.pull()[0], 1);
//     }

//     #[test]
//     fn len() {
//         {
//             let pool = AsyncObjectPool::<Vec<u8>, _, _>::new(|| Vec::new(), |_| {});

//             let object1 = pool.pull();
//             drop(object1);
//             let object2 = pool.pull();
//             drop(object2);

//             assert_eq!(pool.len(), 1);
//         }

//         {
//             let pool = AsyncObjectPool::<Vec<u8>, _, _>::new(|| Vec::new(), |_| {});

//             let object1 = pool.pull();
//             let object2 = pool.pull();

//             drop(object1);
//             drop(object2);
//             assert_eq!(pool.len(), 2);
//         }
//     }

//     #[test]
//     fn e2e() {
//         let pool = AsyncObjectPool::new(|| Vec::new(), |_| {});
//         let mut objects = Vec::new();

//         for i in 0..10 {
//             let mut object = pool.pull();
//             object.push(i);
//             objects.push(object);
//         }

//         drop(objects);

//         for i in (10..0).rev() {
//             let mut object = pool.objects.lock().pop().unwrap();
//             assert_eq!(object.pop(), Some(i));
//         }
//     }

//     #[test]
//     fn reset() {
//         let pool = AsyncObjectPool::new(|| Vec::new(), |v| v.clear());

//         let mut object = pool.pull();
//         object.push(1);
//         drop(object);
//         let object = pool.pull();
//         assert_eq!(object.len(), 0);
//     }

//     #[test]
//     fn no_reset() {
//         let pool = AsyncObjectPool::new(|| Vec::new(), |_| {});

//         let mut object = pool.pull();
//         object.push(1);
//         drop(object);
//         let object = pool.pull();
//         assert_eq!(object.len(), 1);
//     }
// }
