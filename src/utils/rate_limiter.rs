use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

/// RateLimiter controls the number of concurrent API requests and enforces delays
#[derive(Clone)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    min_delay: Duration,
    last_request: Arc<tokio::sync::Mutex<Option<Instant>>>,
}

impl RateLimiter {
    /// Create a new RateLimiter with the specified maximum concurrent requests
    /// and minimum delay between requests
    pub fn new(max_concurrent: usize, min_delay_ms: u64) -> Self {
        RateLimiter {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            min_delay: Duration::from_millis(min_delay_ms),
            last_request: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Acquire a permit to make a request
    /// Returns a guard that will release the permit when dropped
    pub async fn acquire(&self) -> RateLimitGuard {
        // Acquire semaphore permit
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Semaphore closed");

        // Enforce minimum delay between requests
        let mut last_request = self.last_request.lock().await;
        if let Some(last) = *last_request {
            let elapsed = last.elapsed();
            if elapsed < self.min_delay {
                let wait_time = self.min_delay - elapsed;
                tracing::debug!("Rate limiting: waiting {:?}", wait_time);
                sleep(wait_time).await;
            }
        }
        *last_request = Some(Instant::now());
        drop(last_request);

        RateLimitGuard {
            _permit: Some(permit),
        }
    }

    /// Get the current number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// Guard that releases the rate limit permit when dropped
pub struct RateLimitGuard {
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl Drop for RateLimitGuard {
    fn drop(&mut self) {
        // The permit is automatically released when the guard is dropped
        tracing::trace!("Rate limit permit released");
    }
}