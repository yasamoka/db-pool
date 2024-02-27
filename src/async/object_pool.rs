// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::future::Future;
use std::mem::{forget, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

pub type Stack<T> = Vec<T>;

pub struct AsyncObjectPool<T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    objects: Mutex<Stack<T>>,
    init: I,
    reset: R,
}

impl<T, I, R> AsyncObjectPool<T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    #[inline]
    pub fn new(init: I, reset: R) -> AsyncObjectPool<T, I, R> {
        AsyncObjectPool {
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
    pub async fn pull(&self) -> Reusable<T, I, R> {
        let object = self.objects.lock().pop();
        let object = if let Some(object) = object {
            (self.reset)(object).await
        } else {
            (self.init)().await
        };
        Reusable::new(self, object).await
    }

    #[inline]
    pub fn attach(&self, t: T) {
        self.objects.lock().push(t)
    }
}

pub struct Reusable<'b, T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    pool: &'b AsyncObjectPool<T, I, R>,
    data: ManuallyDrop<T>,
}

impl<'b, T, I, R> Reusable<'b, T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    #[inline]
    pub async fn new(pool: &'b AsyncObjectPool<T, I, R>, t: T) -> Self {
        Self {
            pool,
            data: ManuallyDrop::new(t),
        }
    }

    #[inline]
    pub fn detach(mut self) -> (&'b AsyncObjectPool<T, I, R>, T) {
        let ret = unsafe { (self.pool, self.take()) };
        forget(self);
        ret
    }

    unsafe fn take(&mut self) -> T {
        ManuallyDrop::take(&mut self.data)
    }
}

impl<'b, T, I, R> Deref for Reusable<'b, T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'b, T, I, R> DerefMut for Reusable<'b, T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'b, T, I, R> Drop for Reusable<'b, T, I, R>
where
    I: Fn() -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    R: Fn(T) -> Pin<Box<dyn Future<Output = T> + Send + 'static>>,
{
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
