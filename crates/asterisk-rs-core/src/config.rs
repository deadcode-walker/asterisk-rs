//! Shared configuration types used across protocol crates.

use std::time::Duration;

/// reconnection policy with exponential backoff and jitter
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub max_retries: Option<u32>,
    /// multiplicative factor per retry (default 2.0)
    pub backoff_factor: f64,
    /// whether to add random jitter to prevent thundering herd
    pub jitter: bool,
}

impl ReconnectPolicy {
    /// create an exponential backoff policy
    pub fn exponential(initial: Duration, max: Duration) -> Self {
        Self {
            initial_delay: initial,
            max_delay: max,
            max_retries: None,
            backoff_factor: 2.0,
            jitter: true,
        }
    }

    /// create a fixed-interval retry policy
    pub fn fixed(interval: Duration) -> Self {
        Self {
            initial_delay: interval,
            max_delay: interval,
            max_retries: None,
            backoff_factor: 1.0,
            jitter: false,
        }
    }

    /// set maximum number of retries (default: unlimited)
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// disable retry entirely
    pub fn none() -> Self {
        Self {
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            max_retries: Some(0),
            backoff_factor: 1.0,
            jitter: false,
        }
    }

    /// compute delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if self.max_retries.is_some_and(|max| attempt >= max) {
            return Duration::ZERO;
        }
        let base = self.initial_delay.as_secs_f64() * self.backoff_factor.powi(attempt as i32);
        let capped = base.min(self.max_delay.as_secs_f64());

        if self.jitter {
            let jitter_factor = jitter_factor();
            Duration::from_secs_f64(capped * jitter_factor)
        } else {
            Duration::from_secs_f64(capped)
        }
    }
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self::exponential(Duration::from_secs(1), Duration::from_secs(60))
    }
}

/// connection state for health monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "disconnected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Reconnecting => write!(f, "reconnecting"),
        }
    }
}

/// jitter factor between 0.5 and 1.0 using system time for entropy
fn jitter_factor() -> f64 {
    // mix system time nanos with thread id for per-instance variation
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let thread_id = std::thread::current().id();
    let hash = nanos ^ (format!("{thread_id:?}").len() as u32).wrapping_mul(0x9E3779B9);
    let normalized = (hash as f64) / (u32::MAX as f64);
    0.5 + 0.5 * normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff_increases() {
        let policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
        let mut policy = policy;
        policy.jitter = false;

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(8));
    }

    #[test]
    fn exponential_backoff_caps_at_max() {
        let mut policy =
            ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(10));
        policy.jitter = false;

        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(10));
        assert_eq!(policy.delay_for_attempt(100), Duration::from_secs(10));
    }

    #[test]
    fn fixed_policy_constant_delay() {
        let policy = ReconnectPolicy::fixed(Duration::from_secs(5));
        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(5));
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn none_policy_returns_zero() {
        let policy = ReconnectPolicy::none();
        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
    }

    #[test]
    fn max_retries_returns_zero_after_exhausted() {
        let mut policy =
            ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
        policy.jitter = false;
        policy.max_retries = Some(3);

        assert!(policy.delay_for_attempt(2) > Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(3), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(100), Duration::ZERO);
    }

    #[test]
    fn jitter_stays_in_range() {
        let policy = ReconnectPolicy::exponential(Duration::from_secs(10), Duration::from_secs(60));
        let delay = policy.delay_for_attempt(0);
        assert!(delay >= Duration::from_secs(5), "delay too low: {delay:?}");
        assert!(
            delay <= Duration::from_secs(10),
            "delay too high: {delay:?}"
        );
    }

    #[test]
    fn connection_state_display() {
        assert_eq!(ConnectionState::Connected.to_string(), "connected");
        assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionState::Reconnecting.to_string(), "reconnecting");
    }
}
