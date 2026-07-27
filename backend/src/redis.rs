//! Managed Redis integration. Redis is used only for ephemeral distributed
//! coordination; PostgreSQL remains the durable source of truth.

use std::time::Duration;

use redis::{aio::ConnectionManager, AsyncCommands, RedisError, Script};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct RedisStore {
    client: redis::Client,
    manager: ConnectionManager,
    prefix: String,
    command_timeout: Duration,
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
        tokio::time::timeout(self.command_timeout, redis::cmd("PING").query_async::<_, String>(&mut manager))
            .await
            .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "Redis command timed out")))??;
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
        .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "Redis command timed out")))??;
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
        .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "Redis command timed out")))??;
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
        .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "Redis command timed out")))??;
        Ok(())
    }

    pub async fn subscribe_captions(&self, session_id: &str) -> Result<redis::aio::PubSub, RedisError> {
        let mut pubsub = self.client.get_async_pubsub().await?;
        pubsub.subscribe(self.caption_channel(session_id)).await?;
        Ok(pubsub)
    }
}
