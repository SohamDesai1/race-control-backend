use chrono::{DateTime, Utc, Duration};

#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    pub value: T,
    pub expires_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl_seconds: i64) -> Self {
        Self {
            value,
            expires_at: Utc::now() + Duration::seconds(ttl_seconds),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
