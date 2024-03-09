// adapted from https://github.com/CJP10/object-pool and https://github.com/EVaillant/lockfree-object-pool

use parking_lot::Mutex;
use std::future::Future;
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
    data: Option<T>,
}

impl<'a, T> AsyncReusable<'a, T> {
    #[inline]
    pub fn new(pool: &'a AsyncObjectPool<T>, t: T) -> Self {
        Self {
            pool,
            data: Some(t),
        }
    }
}

impl<'a, T> Deref for AsyncReusable<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.as_ref().unwrap()
    }
}

impl<'a, T> DerefMut for AsyncReusable<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().unwrap()
    }
}

impl<'a, T> Drop for AsyncReusable<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.pool.attach(self.data.take().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::AsyncObjectPool;
    use std::mem::drop;

    #[tokio::test]
    async fn len() {
        {
            let pool = AsyncObjectPool::new(
                || Box::pin(async { Vec::<u8>::new() }),
                |obj| Box::pin(async { obj }),
            );

            let object1 = pool.pull().await;
            drop(object1);
            let object2 = pool.pull().await;
            drop(object2);

            assert_eq!(pool.len(), 1);
        }

        {
            let pool = AsyncObjectPool::new(
                || Box::pin(async { Vec::<u8>::new() }),
                |obj| Box::pin(async { obj }),
            );

            let object1 = pool.pull().await;
            let object2 = pool.pull().await;

            drop(object1);
            drop(object2);
            assert_eq!(pool.len(), 2);
        }
    }

    #[tokio::test]
    async fn e2e() {
        let pool = AsyncObjectPool::new(
            || Box::pin(async { Vec::<u8>::new() }),
            |obj| Box::pin(async { obj }),
        );
        let mut objects = Vec::new();

        for i in 0..10 {
            let mut object = pool.pull().await;
            object.push(i);
            objects.push(object);
        }

        drop(objects);

        for i in (0..10).rev() {
            let mut object = pool.objects.lock().pop().unwrap();
            assert_eq!(object.pop(), Some(i));
        }
    }

    #[tokio::test]
    async fn reset() {
        let pool = AsyncObjectPool::new(
            || Box::pin(async { Vec::new() }),
            |mut v| {
                Box::pin(async {
                    v.clear();
                    v
                })
            },
        );

        let mut object = pool.pull().await;
        object.push(1);
        drop(object);
        let object = pool.pull().await;
        assert_eq!(object.len(), 0);
    }

    #[tokio::test]
    async fn no_reset() {
        let pool = AsyncObjectPool::new(
            || Box::pin(async { Vec::new() }),
            |obj| Box::pin(async { obj }),
        );

        let mut object = pool.pull().await;
        object.push(1);
        drop(object);
        let object = pool.pull().await;
        assert_eq!(object.len(), 1);
    }
}
