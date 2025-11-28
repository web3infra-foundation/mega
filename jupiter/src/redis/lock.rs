use common::errors::MegaError;
use redis::{Script, aio::ConnectionManager};
use std::sync::Arc;
use tokio::{
    sync::Notify,
    time::{Duration, sleep},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct RedLock {
    connection: ConnectionManager,
    key: String,
    value: String,
    ttl_ms: u64,
    stop_notify: Arc<Notify>,
}

impl RedLock {
    pub fn new(connection: ConnectionManager, key: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            connection,
            key: key.into(),
            value: Uuid::new_v4().to_string(),
            ttl_ms,
            stop_notify: Arc::new(Notify::new()),
        }
    }

    /// Try lock: SET key value NX PX TTL
    pub async fn try_lock(&self) -> Result<bool, MegaError> {
        // SET returns "OK" or Nil
        let result: Option<String> = redis::cmd("SET")
            .arg(&self.key)
            .arg(&self.value)
            .arg("NX")
            .arg("PX")
            .arg(self.ttl_ms)
            .query_async(&mut self.connection.clone())
            .await?;

        Ok(result.is_some())
    }

    /// Lock with retry
    pub async fn lock(self: Arc<Self>) -> Result<RedLockGuard, MegaError> {
        while !self.try_lock().await? {
            sleep(Duration::from_millis(200)).await;
        }

        self.spawn_auto_renew();

        Ok(RedLockGuard { mutex: self })
    }

    /// Unlock via Lua atomic script
    pub async fn unlock(&self) -> Result<bool, MegaError> {
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

        let mut conn = self.connection.clone();
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

        let mut conn = self.connection.clone();
        tokio::spawn(async move {
            let half = mutex.ttl_ms / 2;

            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(half)) => {},
                    _ = mutex.stop_notify.notified() => break,
                }

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
    pub async fn unlock(self) -> Result<(), MegaError> {
        let mutex = self.mutex.clone();
        mutex.unlock().await?;
        Ok(())
    }
}
