use compact_str::CompactString;
use drop_bomb::DropBomb;
use redis::aio::{ConnectionLike, ConnectionManager};
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions, Value};
use std::iter::repeat_with;

#[derive(Clone)]
pub struct RedisLock {
    rconn: ConnectionManager,
    /// Value the lock is tied to
    resource: i32,
    /// A random alphanumeric string
    value: CompactString,
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
    pub fn new(rconn: ConnectionManager, resource: i32, ttl_max: usize) -> RedisLock {
        let lock_val: CompactString = repeat_with(fastrand::alphanumeric).take(10).collect();

        Self {
            rconn,
            value: lock_val,
            resource,
            ttl_max,
        }
    }
    pub async fn acquire(&self) -> Result<RedisLockGuard, ()> {
        loop {
            match lock(self.rconn.clone(), self.resource, &self.value, self.ttl_max).await {
                true => {
                    eprintln!("got lock");
                    return Ok(RedisLockGuard {
                        lock: self,
                        bomb: DropBomb::new("RedisLockGuard must be released with release."),
                    });
                }
                false => tokio::task::yield_now().await,
            };
        }
    }

    async fn unlock(&self) -> Result<bool, ()> {
        let res = drop_lock(self.rconn.clone(), self.resource, &self.value).await;
        eprintln!(
            "droppped lock for user: {}, resource: {}",
            self.resource, self.value
        );
        Ok(res)
    }
}

async fn lock(
    mut redis: impl AsyncCommands,
    resource: i32,
    val: &str,
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
    mut conn: impl AsyncCommands,
    resource: i32,
    val: &str,
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
