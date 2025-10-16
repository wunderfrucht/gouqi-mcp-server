//! Rate Limiter - Sliding Window Implementation
//!
//! Similar to Atlassian's approach: prevents hitting rate limits by queuing requests
//! Uses a sliding window to track requests over time

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Sliding window rate limiter
/// Tracks request timestamps and enforces limits by blocking requests
#[derive(Clone)]
pub struct RateLimiter {
    /// Request timestamps within the current window
    state: Arc<Mutex<RateLimiterState>>,
    /// Maximum requests allowed per window
    max_requests: usize,
    /// Time window duration
    window_duration: Duration,
}

struct RateLimiterState {
    /// Timestamps of recent requests
    request_times: Vec<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests per window
    /// * `window_duration` - Duration of the sliding window
    ///
    /// # Example
    /// ```
    /// // 100 requests per 60 seconds (similar to JIRA Cloud limits)
    /// let limiter = RateLimiter::new(100, Duration::from_secs(60));
    /// ```
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                request_times: Vec::with_capacity(max_requests),
            })),
            max_requests,
            window_duration,
        }
    }

    /// Wait for a slot to become available
    ///
    /// This method blocks until a request slot is available, ensuring we never
    /// exceed the rate limit. Uses a sliding window algorithm.
    pub async fn wait_for_slot(&self) {
        loop {
            let mut state = self.state.lock().await;
            let now = Instant::now();

            // Remove timestamps older than the window
            state
                .request_times
                .retain(|&time| now.duration_since(time) < self.window_duration);

            // Check if we have capacity
            if state.request_times.len() < self.max_requests {
                // Add current timestamp and allow the request
                state.request_times.push(now);
                debug!(
                    "Rate limiter: Allowed request ({}/{} used)",
                    state.request_times.len(),
                    self.max_requests
                );
                return;
            }

            // Calculate how long to wait
            let oldest_request = state.request_times[0];
            let elapsed = now.duration_since(oldest_request);
            let wait_time =
                self.window_duration.saturating_sub(elapsed) + Duration::from_millis(100);

            warn!(
                "Rate limiter: Limit reached ({}/{}), waiting {:?}",
                state.request_times.len(),
                self.max_requests,
                wait_time
            );

            // Release the lock before sleeping
            drop(state);

            // Wait before trying again
            tokio::time::sleep(wait_time).await;
        }
    }

    /// Get current usage statistics
    pub async fn get_stats(&self) -> RateLimiterStats {
        let state = self.state.lock().await;
        let now = Instant::now();

        // Count requests in current window
        let active_requests = state
            .request_times
            .iter()
            .filter(|&&time| now.duration_since(time) < self.window_duration)
            .count();

        RateLimiterStats {
            active_requests,
            max_requests: self.max_requests,
            window_duration: self.window_duration,
            utilization_percent: (active_requests as f64 / self.max_requests as f64 * 100.0),
        }
    }

    /// Reset the rate limiter (clear all timestamps)
    #[allow(dead_code)]
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.request_times.clear();
        debug!("Rate limiter reset");
    }
}

/// Statistics about rate limiter usage
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub active_requests: usize,
    pub max_requests: usize,
    pub window_duration: Duration,
    pub utilization_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        // Should allow 5 requests immediately
        for i in 0..5 {
            limiter.wait_for_slot().await;
            println!("Request {} allowed", i + 1);
        }

        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_requests, 5);
        assert_eq!(stats.utilization_percent, 100.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_millis(500));

        // Allow 3 requests
        for _ in 0..3 {
            limiter.wait_for_slot().await;
        }

        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_requests, 3);

        // 4th request should wait
        let start = Instant::now();
        limiter.wait_for_slot().await;
        let elapsed = start.elapsed();

        // Should have waited at least 400ms (500ms window - some buffer)
        assert!(elapsed >= Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_sliding_window() {
        let limiter = RateLimiter::new(2, Duration::from_millis(500));

        // First request
        limiter.wait_for_slot().await;

        // Wait 300ms
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Second request (should be allowed)
        limiter.wait_for_slot().await;

        // Stats should show 2 active requests
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_requests, 2);

        // Wait another 300ms (first request is now > 500ms old)
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Third request should be allowed now (first one expired)
        let start = Instant::now();
        limiter.wait_for_slot().await;
        let elapsed = start.elapsed();

        // Should not have waited (or very little)
        assert!(elapsed < Duration::from_millis(100));
    }
}
