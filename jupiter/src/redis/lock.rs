use redis::{Client, RedisResult, Script};
use std::sync::Arc;
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct RedLock {
    client: Arc<Client>,
    key: String,
    value: String,
    ttl_ms: u64,
    stop_notify: Arc<Notify>,
}

impl RedLock {
    pub fn new(client: Arc<Client>, key: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            client,
            key: key.into(),
            value: Uuid::new_v4().to_string(),
            ttl_ms,
            stop_notify: Arc::new(Notify::new()),
        }
    }

    /// Try lock: SET key value NX PX TTL
    pub async fn try_lock(&self) -> redis::RedisResult<bool> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // SET returns "OK" or Nil
        let result: Option<String> = redis::cmd("SET")
            .arg(&self.key)
            .arg(&self.value)
            .arg("NX")
            .arg("PX")
            .arg(self.ttl_ms)
            .query_async(&mut conn)
            .await?;

        Ok(result.is_some())
    }

    /// Lock with retry
    pub async fn lock(self: Arc<Self>) -> redis::RedisResult<RedLockGuard> {
        while !self.try_lock().await? {
            sleep(Duration::from_millis(200)).await;
        }

        self.spawn_auto_renew();

        Ok(RedLockGuard { mutex: self })
    }

    /// Unlock via Lua atomic script
    pub async fn unlock(&self) -> redis::RedisResult<bool> {
        // stop renewer task
        self.stop_notify.notify_waiters();

        let script = Script::new(
            r#"
                if redis.call("GET", KEYS[1]) == ARGV[1] then
                    return redis.call("DEL", KEYS[1])
                else
                    return 0
                end
            "#,
        );

        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let deleted: i32 = script
            .key(&self.key)
            .arg(&self.value)
            .invoke_async::<_>(&mut conn)
            .await?;

        Ok(deleted == 1)
    }

    /// Spawn TTL renew task
    fn spawn_auto_renew(self: &Arc<Self>) {
        let mutex = Arc::clone(self);

        tokio::spawn(async move {
            let half = mutex.ttl_ms / 2;

            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(half)) => {},
                    _ = mutex.stop_notify.notified() => break,
                }

                let mut conn = match mutex.client.get_multiplexed_async_connection().await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // PEXPIRE returns integer reply
                let _: () = redis::cmd("PEXPIRE")
                    .arg(&mutex.key)
                    .arg(mutex.ttl_ms)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or(());
            }
        });
    }
}

pub struct RedLockGuard {
    mutex: Arc<RedLock>,
}

impl RedLockGuard {
    pub async fn unlock(self) -> RedisResult<()> {
        let mutex = self.mutex.clone();
        mutex.unlock().await?;
        Ok(())
    }
}
