use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Performance metrics collector for the web server
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_response_time_ms: AtomicU64,
    active_sessions: AtomicU64,
    total_events_broadcast: AtomicU64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                total_requests: AtomicU64::new(0),
                successful_requests: AtomicU64::new(0),
                failed_requests: AtomicU64::new(0),
                total_response_time_ms: AtomicU64::new(0),
                active_sessions: AtomicU64::new(0),
                total_events_broadcast: AtomicU64::new(0),
            }),
        }
    }

    /// Record a successful HTTP request
    pub fn record_request_success(&self, duration_ms: u64) {
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
        self.inner
            .successful_requests
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .total_response_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        tracing::trace!(duration_ms = duration_ms, "recorded successful request");
    }

    /// Record a failed HTTP request
    pub fn record_request_failure(&self, duration_ms: u64) {
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
        self.inner.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.inner
            .total_response_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        tracing::trace!(duration_ms = duration_ms, "recorded failed request");
    }

    /// Increment active session count
    pub fn increment_active_sessions(&self) {
        let count = self.inner.active_sessions.fetch_add(1, Ordering::Relaxed) + 1;
        tracing::debug!(active_sessions = count, "session count increased");
    }

    /// Decrement active session count
    pub fn decrement_active_sessions(&self) {
        let mut current = self.inner.active_sessions.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                tracing::warn!("attempted to decrement active_sessions below zero");
                return;
            }

            match self.inner.active_sessions.compare_exchange(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    tracing::debug!(active_sessions = current - 1, "session count decreased");
                    return;
                }
                Err(actual) => current = actual,
            }
        }
    }

    /// Record an event broadcast
    pub fn record_event_broadcast(&self) {
        self.inner
            .total_events_broadcast
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.inner.total_requests.load(Ordering::Relaxed),
            successful_requests: self.inner.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.inner.failed_requests.load(Ordering::Relaxed),
            total_response_time_ms: self.inner.total_response_time_ms.load(Ordering::Relaxed),
            active_sessions: self.inner.active_sessions.load(Ordering::Relaxed),
            total_events_broadcast: self.inner.total_events_broadcast.load(Ordering::Relaxed),
        }
    }

    /// Log current metrics
    pub fn log_metrics(&self) {
        let snapshot = self.snapshot();
        let avg_response_time = if snapshot.total_requests > 0 {
            snapshot.total_response_time_ms / snapshot.total_requests
        } else {
            0
        };

        tracing::info!(
            total_requests = snapshot.total_requests,
            successful_requests = snapshot.successful_requests,
            failed_requests = snapshot.failed_requests,
            avg_response_time_ms = avg_response_time,
            active_sessions = snapshot.active_sessions,
            total_events_broadcast = snapshot.total_events_broadcast,
            "performance metrics"
        );
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_response_time_ms: u64,
    pub active_sessions: u64,
    pub total_events_broadcast: u64,
}

impl MetricsSnapshot {
    pub fn average_response_time_ms(&self) -> u64 {
        if self.total_requests > 0 {
            self.total_response_time_ms / self.total_requests
        } else {
            0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64) / (self.total_requests as f64)
        } else {
            0.0
        }
    }
}

/// RAII guard for timing requests
pub struct RequestTimer {
    start: Instant,
    metrics: MetricsCollector,
}

impl RequestTimer {
    pub fn new(metrics: MetricsCollector) -> Self {
        Self {
            start: Instant::now(),
            metrics,
        }
    }

    pub fn finish_success(self) {
        let duration = self.start.elapsed().as_millis() as u64;
        self.metrics.record_request_success(duration);
    }

    pub fn finish_failure(self) {
        let duration = self.start.elapsed().as_millis() as u64;
        self.metrics.record_request_failure(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_creation() {
        let metrics = MetricsCollector::new();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.total_requests, 0);
        assert_eq!(snapshot.successful_requests, 0);
        assert_eq!(snapshot.failed_requests, 0);
        assert_eq!(snapshot.active_sessions, 0);
        assert_eq!(snapshot.total_events_broadcast, 0);
    }

    #[test]
    fn test_record_successful_request() {
        let metrics = MetricsCollector::new();
        metrics.record_request_success(100);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.successful_requests, 1);
        assert_eq!(snapshot.failed_requests, 0);
        assert_eq!(snapshot.total_response_time_ms, 100);
    }

    #[test]
    fn test_record_failed_request() {
        let metrics = MetricsCollector::new();
        metrics.record_request_failure(50);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.successful_requests, 0);
        assert_eq!(snapshot.failed_requests, 1);
        assert_eq!(snapshot.total_response_time_ms, 50);
    }

    #[test]
    fn test_session_count_management() {
        let metrics = MetricsCollector::new();

        metrics.increment_active_sessions();
        assert_eq!(metrics.snapshot().active_sessions, 1);

        metrics.increment_active_sessions();
        assert_eq!(metrics.snapshot().active_sessions, 2);

        metrics.decrement_active_sessions();
        assert_eq!(metrics.snapshot().active_sessions, 1);

        metrics.decrement_active_sessions();
        assert_eq!(metrics.snapshot().active_sessions, 0);
    }

    #[test]
    fn test_event_broadcast_recording() {
        let metrics = MetricsCollector::new();

        metrics.record_event_broadcast();
        metrics.record_event_broadcast();
        metrics.record_event_broadcast();

        assert_eq!(metrics.snapshot().total_events_broadcast, 3);
    }

    #[test]
    fn test_average_response_time() {
        let metrics = MetricsCollector::new();

        metrics.record_request_success(100);
        metrics.record_request_success(200);
        metrics.record_request_success(300);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.average_response_time_ms(), 200);
    }

    #[test]
    fn test_success_rate() {
        let metrics = MetricsCollector::new();

        metrics.record_request_success(100);
        metrics.record_request_success(100);
        metrics.record_request_failure(100);

        let snapshot = metrics.snapshot();
        assert!((snapshot.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_request_timer() {
        let metrics = MetricsCollector::new();

        {
            let timer = RequestTimer::new(metrics.clone());
            thread::sleep(Duration::from_millis(10));
            timer.finish_success();
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.successful_requests, 1);
        assert!(snapshot.total_response_time_ms >= 10);
    }

    #[test]
    fn test_concurrent_metric_updates() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let m = Arc::clone(&metrics);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    m.record_request_success(1);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1000);
        assert_eq!(snapshot.successful_requests, 1000);
    }
}
