//! Managed Redis integration. Redis is used only for ephemeral distributed
//! coordination; PostgreSQL remains the durable source of truth.

use std::time::Duration;

use redis::{aio::ConnectionManager, streams::{StreamReadOptions, StreamReadReply}, AsyncCommands, RedisError, Script};
use uuid::Uuid;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct RedisStore {
    client: redis::Client,
    manager: ConnectionManager,
    prefix: String,
    command_timeout: Duration,
}

#[derive(Clone)]
pub struct RedisLock {
    store: RedisStore,
    key: String,
    token: String,
}

impl RedisStore {
    pub async fn connect(config: &AppConfig) -> Result<Self, RedisError> {
        let url = config.redis_url.as_deref().ok_or_else(|| {
            RedisError::from((redis::ErrorKind::InvalidClientConfig, "REDIS_URL is not configured"))
        })?;
        let client = redis::Client::open(url)?;
        let manager = client.get_connection_manager().await?;
        Ok(Self {
            client,
            manager,
            prefix: config.redis_key_prefix.clone(),
            command_timeout: Duration::from_millis(config.redis_command_timeout_ms),
        })
    }

    pub fn key(&self, scope: &str, identity: &str) -> String {
        format!("{}:{}:{}", self.prefix, scope, identity)
    }

    pub async fn ping(&self) -> Result<(), RedisError> {
        let mut manager = self.manager.clone();
        tokio::time::timeout(self.command_timeout, redis::cmd("PING").query_async::<String>(&mut manager))
            .await
            .map_err(|_| RedisError::from((redis::ErrorKind::Io, "Redis command timed out")))??;
        Ok(())
    }

    /// Atomically increments a fixed-window counter and applies its expiry on
    /// the first request. This is safe across all API instances.
    pub async fn consume_rate_limit(
        &self,
        scope: &str,
        identity: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<bool, RedisError> {
        let key = self.key("ratelimit", &format!("{scope}:{identity}"));
        let script = Script::new(
            "local current = redis.call('INCR', KEYS[1]); if current == 1 then redis.call('EXPIRE', KEYS[1], ARGV[1]) end; return current",
        );
        let mut manager = self.manager.clone();
        let current: u32 = tokio::time::timeout(
            self.command_timeout,
            script.key(key).arg(window_seconds).invoke_async(&mut manager),
        )
        .await
        .map_err(|_| RedisError::from((redis::ErrorKind::Io, "Redis command timed out")))??;
        Ok(current <= limit)
    }

    pub async fn set_presence(
        &self,
        session_id: &str,
        participant_id: &str,
        ttl_seconds: u64,
    ) -> Result<(), RedisError> {
        let key = self.key("presence", &format!("{session_id}:{participant_id}"));
        let mut manager = self.manager.clone();
        tokio::time::timeout(
            self.command_timeout,
            manager.set_ex::<_, _, ()>(key, "1", ttl_seconds),
        )
        .await
        .map_err(|_| RedisError::from((redis::ErrorKind::Io, "Redis command timed out")))??;
        Ok(())
    }

    pub fn caption_channel(&self, session_id: &str) -> String {
        self.key("captions", session_id)
    }

    pub async fn publish_caption(&self, session_id: &str, payload: &str) -> Result<(), RedisError> {
        let mut manager = self.manager.clone();
        tokio::time::timeout(
            self.command_timeout,
            manager.publish::<_, _, ()>(self.caption_channel(session_id), payload),
        )
        .await
        .map_err(|_| RedisError::from((redis::ErrorKind::Io, "Redis command timed out")))??;
        Ok(())
    }

    pub async fn subscribe_captions(&self, session_id: &str) -> Result<redis::aio::PubSub, RedisError> {
        let mut pubsub = self.client.get_async_pubsub().await?;
        pubsub.subscribe(self.caption_channel(session_id)).await?;
        Ok(pubsub)
    }

    pub async fn enqueue_ai_job(&self, job_id: &str) -> Result<(), RedisError> {
        let stream = self.key("ai-jobs", "stream");
        let mut manager = self.manager.clone();
        tokio::time::timeout(
            self.command_timeout,
            redis::cmd("XADD")
                .arg(stream)
                .arg("MAXLEN").arg("~").arg(10000)
                .arg("*").arg("job_id").arg(job_id)
                .query_async::<String>(&mut manager),
        )
        .await
        .map_err(|_| RedisError::from((redis::ErrorKind::Io, "Redis command timed out")))??;
        Ok(())
    }

    pub async fn ensure_ai_consumer_group(&self) -> Result<(), RedisError> {
        let stream = self.key("ai-jobs", "stream");
        let mut manager = self.manager.clone();
        let result: redis::RedisResult<String> = redis::cmd("XGROUP")
            .arg("CREATE").arg(stream).arg("ai-workers").arg("$").arg("MKSTREAM")
            .query_async(&mut manager).await;
        match result {
            Ok(_) => Ok(()),
            Err(error) if error.to_string().contains("BUSYGROUP") => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub async fn read_ai_job(&self, consumer: &str) -> Result<Option<(String, String)>, RedisError> {
        let stream = self.key("ai-jobs", "stream");
        let options = StreamReadOptions::default().group("ai-workers", consumer).count(1).block(1000);
        let mut manager = self.manager.clone();
        let reply: StreamReadReply = manager.xread_options(&[stream], &[">"], &options).await?;
        for key in reply.keys {
            for entry in key.ids {
                if let Some(value) = entry.map.get("job_id") {
                    if let Ok(job_id) = redis::from_redis_value::<String>(value.clone()) {
                        return Ok(Some((entry.id, job_id)));
                    }
                }
            }
        }
        Ok(None)
    }

    pub async fn acknowledge_ai_job(&self, message_id: &str) -> Result<(), RedisError> {
        let stream = self.key("ai-jobs", "stream");
        let mut manager = self.manager.clone();
        manager.xack::<_, _, _, ()>(stream, "ai-workers", &[message_id]).await
    }

    pub async fn try_lock(&self, scope: &str, identity: &str, ttl_seconds: u64) -> Result<Option<RedisLock>, RedisError> {
        let key = self.key("lock", &format!("{scope}:{identity}"));
        let token = Uuid::now_v7().to_string();
        let script = Script::new("if redis.call('SET', KEYS[1], ARGV[1], 'NX', 'EX', ARGV[2]) then return 1 else return 0 end");
        let mut manager = self.manager.clone();
        let acquired: i32 = script.key(&key).arg(&token).arg(ttl_seconds).invoke_async(&mut manager).await?;
        if acquired == 1 { Ok(Some(RedisLock { store: self.clone(), key, token })) } else { Ok(None) }
    }

    pub async fn reserve_idempotency(&self, scope: &str, identity: &str, ttl_seconds: u64) -> Result<bool, RedisError> {
        let key = self.key("idempotency", &format!("{scope}:{identity}"));
        let mut manager = self.manager.clone();
        let result: Option<String> = redis::cmd("SET")
            .arg(key).arg("1").arg("NX").arg("EX").arg(ttl_seconds)
            .query_async(&mut manager).await?;
        Ok(result.is_some())
    }
}

impl RedisLock {
    pub async fn release(self) -> Result<(), RedisError> {
        let script = Script::new("if redis.call('GET', KEYS[1]) == ARGV[1] then return redis.call('DEL', KEYS[1]) else return 0 end");
        let mut manager = self.store.manager.clone();
        let _: i32 = script.key(self.key).arg(self.token).invoke_async(&mut manager).await?;
        Ok(())
    }
}
