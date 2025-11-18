use std::marker::PhantomData;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::Level;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, Registry};

/// Structured log entry for testing and analysis
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

/// Test subscriber that captures log entries for verification
#[derive(Debug, Clone)]
pub struct TestLogSubscriber {
    entries: Arc<Mutex<Vec<LogEntry>>>,
}

impl Default for TestLogSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl TestLogSubscriber {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn into_layer<S>(self) -> TestLayer<S>
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    {
        TestLayer {
            subscriber: self,
            _phantom: PhantomData,
        }
    }
}

pub struct TestLayer<S> {
    subscriber: TestLogSubscriber,
    _phantom: PhantomData<S>,
}

impl<S> Layer<S> for TestLayer<S>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: visitor.fields,
        };

        self.subscriber.entries.lock().unwrap().push(entry);
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        if field.name() == "message" {
            self.message = Some(value_str);
        } else {
            self.fields.push((field.name().to_string(), value_str));
        }
    }
}

/// Initialize logging for the application
pub fn init_logging() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,axiomind_web=debug"));

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

/// Initialize test logging with a custom subscriber
pub fn init_test_logging() -> TestLogSubscriber {
    static SUBSCRIBER: OnceLock<TestLogSubscriber> = OnceLock::new();
    static REGISTERED: OnceLock<()> = OnceLock::new();

    let subscriber = SUBSCRIBER.get_or_init(TestLogSubscriber::new);

    REGISTERED.get_or_init(|| {
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);
        tracing::subscriber::set_global_default(registry)
            .expect("Failed to set global default test subscriber");
    });

    subscriber.clear();
    subscriber.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    #[test]
    fn test_log_subscriber_captures_entries() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            info!("test info message");
            warn!("test warning message");
            error!("test error message");
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, Level::INFO);
        assert!(entries[0].message.contains("test info message"));
        assert_eq!(entries[1].level, Level::WARN);
        assert!(entries[1].message.contains("test warning message"));
        assert_eq!(entries[2].level, Level::ERROR);
        assert!(entries[2].message.contains("test error message"));
    }

    #[test]
    fn test_log_subscriber_captures_fields() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            info!(session_id = "abc123", user = "test", "session created");
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.contains("session created"));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "session_id" && v.contains("abc123")));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "user" && v.contains("test")));
    }

    #[test]
    fn test_log_subscriber_clear() {
        let subscriber = TestLogSubscriber::new();

        // First log
        let layer1 = subscriber.clone().into_layer::<Registry>();
        let registry1 = Registry::default().with(layer1);
        tracing::subscriber::with_default(registry1, || {
            info!("first message");
        });
        assert_eq!(subscriber.entries().len(), 1);

        // Clear logs
        subscriber.clear();
        assert_eq!(subscriber.entries().len(), 0);

        // Second log with new registry
        let layer2 = subscriber.clone().into_layer::<Registry>();
        let registry2 = Registry::default().with(layer2);
        tracing::subscriber::with_default(registry2, || {
            info!("second message");
        });
        assert_eq!(subscriber.entries().len(), 1);
    }

    #[test]
    fn test_different_log_levels() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            debug!("debug message");
            info!("info message");
            warn!("warn message");
            error!("error message");
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].level, Level::DEBUG);
        assert_eq!(entries[1].level, Level::INFO);
        assert_eq!(entries[2].level, Level::WARN);
        assert_eq!(entries[3].level, Level::ERROR);
    }
}
