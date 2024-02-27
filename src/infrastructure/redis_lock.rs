use compact_str::CompactString;
use redis::{Client, Value};
use std::iter::repeat_with;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RedisLock {
    rconn: Arc<Client>,
    /// A random alphanumeric string
    resource: CompactString,
    /// Value the lock is tied to
    val: i32,
    ttl_max: usize,
}

#[derive(Debug, Clone)]
pub struct RedisLockGuard<'a> {
    lock: &'a RedisLock,
}

impl<'a> Drop for RedisLockGuard<'a> {
    fn drop(&mut self) {
        futures::executor::block_on(async { self.lock.unlock().await });
    }
}

impl RedisLock {
    pub fn new(rconn: impl Into<Arc<Client>>, val: i32, ttl_max: usize) -> RedisLock {
        let resource: CompactString = repeat_with(fastrand::alphanumeric).take(5).collect();

        Self {
            rconn: rconn.into(),
            resource,
            val,
            ttl_max,
        }
    }
    pub async fn acquire(&self) -> Result<RedisLockGuard, ()> {
        if lock(&self.rconn, &self.resource, self.val, self.ttl_max).await {
            Ok(RedisLockGuard { lock: self })
        } else {
            return Err(());
        }
    }

    async fn unlock(&self) -> Result<(), ()> {
        let res = drop_lock(&self.rconn, &self.resource, self.val).await;
        Ok(())
    }
}

async fn lock(redis: &Client, resource: &str, val: i32, ttl: usize) -> bool {
    let mut conn = redis.get_tokio_connection().await.unwrap();
    let result = redis::cmd("SET")
        .arg(resource)
        .arg(val)
        .arg("NX")
        .arg("PX")
        .arg(ttl)
        .query_async::<_, Value>(&mut conn)
        .await
        .unwrap();
    matches!(result, Value::Okay)
}

async fn drop_lock(redis: &Client, resource: &str, val: i32) -> bool {
    let mut connection = redis.get_tokio_connection().await.unwrap();
    let script = redis::Script::new(DROP_SCRIPT);
    let res = script
        .key(resource)
        .arg(val)
        .invoke_async::<_, Value>(&mut connection)
        .await
        .unwrap();
    matches!(res, Value::Okay)
}

const DROP_SCRIPT: &str = r#"if redis.call("get", KEYS[1]) == ARGV[1] then
        return redis.call("del", KEYS[1])
    else
        return 0
    end"#;
