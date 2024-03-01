use compact_str::CompactString;
use drop_bomb::DropBomb;
use redis::aio::{ConnectionLike, ConnectionManager};
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions, Value};
use std::iter::repeat_with;

#[derive(Clone)]
pub struct RedisLock {
    rconn: ConnectionManager,
    /// A random alphanumeric string
    resource: CompactString,
    /// Value the lock is tied to
    val: i32,
    ttl_max: usize,
}

pub struct RedisLockGuard<'a> {
    lock: &'a RedisLock,
    bomb: DropBomb,
}

impl<'a> RedisLockGuard<'a> {
    pub async fn release(mut self) {
        self.lock.unlock().await.unwrap();
        self.bomb.defuse();
    }
}

impl RedisLock {
    pub fn new(rconn: ConnectionManager, val: i32, ttl_max: usize) -> RedisLock {
        let resource: CompactString = repeat_with(fastrand::alphanumeric).take(5).collect();

        Self {
            rconn,
            resource,
            val,
            ttl_max,
        }
    }
    pub async fn acquire(&self) -> Result<RedisLockGuard, ()> {
        let mut retries = 0;
        loop {
            if retries > 10 {
                return Err(());
            }

            match lock(self.rconn.clone(), &self.resource, self.val, self.ttl_max).await {
                true => {
                    return Ok(RedisLockGuard {
                        lock: self,
                        bomb: DropBomb::new("RedisLockGuard must be released with release."),
                    });
                }
                false => tokio::task::yield_now().await,
            };
            retries += 1;
        }
    }

    async fn unlock(&self) -> Result<bool, ()> {
        let res = drop_lock(self.rconn.clone(), &self.resource, self.val).await;
        Ok(res)
    }
}

async fn lock(
    mut redis: impl ConnectionLike + AsyncCommands,
    resource: &str,
    val: i32,
    ttl: usize,
) -> bool {
    let options = SetOptions::default()
        .conditional_set(ExistenceCheck::NX)
        .with_expiration(SetExpiry::PX(ttl));

    let res = redis
        .set_options::<_, _, Value>(resource, val, options)
        .await
        .unwrap();
    matches!(res, Value::Okay)
}

async fn drop_lock(
    mut conn: impl ConnectionLike + AsyncCommands,
    resource: &str,
    val: i32,
) -> bool {
    let script = redis::Script::new(DROP_SCRIPT);
    let res = script
        .key(resource)
        .arg(val)
        .invoke_async::<_, i32>(&mut conn)
        .await;

    match res {
        Ok(val) => val == 1,
        Err(_) => false,
    }
}

const DROP_SCRIPT: &str = r#"
    if redis.call("get", KEYS[1]) == ARGV[1] then
        return redis.call("del", KEYS[1])
    else
        return 0
    end"#;
