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
    /// connected duration required before the retry budget resets
    pub stability_window: Duration,
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
            stability_window: Duration::from_secs(30),
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
            stability_window: Duration::from_secs(30),
        }
    }

    /// set maximum number of retries (default: unlimited)
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// set the connected duration required before the retry budget resets
    pub fn with_stability_window(mut self, duration: Duration) -> Self {
        self.stability_window = duration;
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
            stability_window: Duration::ZERO,
        }
    }

    /// validate that this policy cannot create an invalid or hot retry loop
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_retries == Some(0) {
            return Ok(());
        }
        if self.initial_delay.is_zero() {
            return Err("initial_delay must be greater than zero when reconnect is enabled");
        }
        if self.max_delay < self.initial_delay {
            return Err("max_delay must be greater than or equal to initial_delay");
        }
        if !self.backoff_factor.is_finite() || self.backoff_factor < 1.0 {
            return Err("backoff_factor must be finite and at least 1.0");
        }
        if self.stability_window.is_zero() {
            return Err("stability_window must be greater than zero when reconnect is enabled");
        }
        if std::time::Instant::now()
            .checked_add(self.stability_window)
            .is_none()
        {
            return Err("stability_window is too large");
        }
        Ok(())
    }

    /// compute delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if self.max_retries.is_some_and(|max| attempt >= max) {
            return Duration::ZERO;
        }
        let max = self.max_delay.as_secs_f64();
        let base = self.initial_delay.as_secs_f64() * self.backoff_factor.powf(f64::from(attempt));
        let capped = base.max(0.0).min(max);

        if self.jitter {
            let jitter_factor = jitter_factor();
            Duration::from_secs_f64((capped * jitter_factor).min(max))
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

/// jitter factor in [0.5, 1.5) using OS-seeded entropy via RandomState
fn jitter_factor() -> f64 {
    use std::hash::{BuildHasher, Hasher};

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);

    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(nanos);
    let hash = hasher.finish();

    0.5 + (hash % 1000) as f64 / 1000.0
}
