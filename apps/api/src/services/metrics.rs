//! Dependency-free Prometheus text-format metrics registry.
//! Label cardinality is strictly low (status/action/type), never user/topic ids.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const REVIEW_BUCKETS: [f64; 7] = [1.0, 15.0, 60.0, 300.0, 3600.0, 86400.0, 604800.0];

#[derive(Default)]
struct MetricEntry {
    value: AtomicU64,
}

#[derive(Default)]
struct Histogram {
    buckets: Mutex<Vec<(f64, AtomicU64)>>,
    sum_seconds: AtomicU64,
    count: AtomicU64,
}

#[derive(Default)]
struct MetricsInner {
    counters: Mutex<HashMap<String, MetricEntry>>,
    review_duration: Histogram,
}

#[derive(Default, Clone)]
pub struct MetricsRegistry {
    inner: Arc<MetricsInner>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Self {
            inner: Arc::new(MetricsInner {
                counters: Mutex::new(HashMap::new()),
                review_duration: Histogram {
                    buckets: Mutex::new(
                        REVIEW_BUCKETS
                            .iter()
                            .map(|le| (*le, AtomicU64::new(0)))
                            .collect(),
                    ),
                    sum_seconds: AtomicU64::new(0),
                    count: AtomicU64::new(0),
                },
            }),
        };
        // Pre-register well-known counters so they appear even at zero.
        for name in [
            "http_requests_total",
            "moderation_reports_total",
            "moderation_actions_total",
            "moderation_auto_rules_triggered_total",
            "moderation_appeals_total",
            "moderation_websocket_events_total",
        ] {
            registry
                .inner
                .counters
                .lock()
                .unwrap()
                .entry(name.to_owned())
                .or_insert_with(|| MetricEntry {
                    value: AtomicU64::new(0),
                });
        }
        registry
    }

    /// Increment a labeled counter. `name` must be a pre-registered metric name;
    /// labels are appended as a `{k="v",...}` suffix.
    pub fn inc(&self, name: &'static str, labels: &[(&str, &str)]) {
        let mut counters = self.inner.counters.lock().unwrap();
        // Labels are encoded into the key; only low-cardinality values are accepted by callers.
        let key = if labels.is_empty() {
            name.to_owned()
        } else {
            let joined = labels
                .iter()
                .map(|(key, value)| format!("{key}=\"{}\"", sanitize(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{name}{{{joined}}}")
        };
        let entry = counters.entry(key).or_insert_with(|| MetricEntry {
            value: AtomicU64::new(0),
        });
        entry.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn observe_review_duration(&self, seconds: f64) {
        let seconds = seconds.max(0.0);
        let mut buckets = self.inner.review_duration.buckets.lock().unwrap();
        for (le, count) in buckets.iter_mut() {
            if seconds <= *le {
                count.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.inner
            .review_duration
            .sum_seconds
            .fetch_add((seconds * 1000.0) as u64, Ordering::Relaxed);
        self.inner
            .review_duration
            .count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Render the registry in Prometheus text exposition format.
    /// Read a counter value (0 when absent) — used by the admin dashboard.
    pub fn counter_value(&self, name: &'static str) -> u64 {
        let counters = self.inner.counters.lock().unwrap();
        counters
            .get(name)
            .map(|entry| entry.value.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut grouped: HashMap<String, Vec<(String, u64)>> = HashMap::new();
        {
            let counters = self.inner.counters.lock().unwrap();
            for (key, entry) in counters.iter() {
                let value = entry.value.load(Ordering::Relaxed);
                if let Some(open) = key.find('{') {
                    let name = key[..open].to_owned();
                    let labels = key[open..].to_owned();
                    grouped.entry(name).or_default().push((labels, value));
                } else {
                    grouped
                        .entry(key.clone())
                        .or_default()
                        .push((String::new(), value));
                }
            }
        }
        let mut names: Vec<String> = grouped.keys().cloned().collect();
        names.sort_unstable();
        for name in names {
            let help = help_text(&name);
            out.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
            let mut series = grouped.get(&name).unwrap().clone();
            series.sort();
            for (labels, value) in series {
                out.push_str(&format!("{name}{labels} {value}\n"));
            }
        }

        // Review duration histogram
        out.push_str(
            "# HELP moderation_review_duration_seconds Time between report creation and handling\n",
        );
        out.push_str("# TYPE moderation_review_duration_seconds histogram\n");
        let buckets = self.inner.review_duration.buckets.lock().unwrap();
        for (le, count) in buckets.iter() {
            out.push_str(&format!(
                "moderation_review_duration_seconds_bucket{{le=\"{le}\"}} {}\n",
                count.load(Ordering::Relaxed)
            ));
        }
        out.push_str(&format!(
            "moderation_review_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            self.inner.review_duration.count.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "moderation_review_duration_seconds_sum {:.3}\n",
            self.inner
                .review_duration
                .sum_seconds
                .load(Ordering::Relaxed) as f64
                / 1000.0
        ));
        out.push_str(&format!(
            "moderation_review_duration_seconds_count {}\n",
            self.inner.review_duration.count.load(Ordering::Relaxed)
        ));
        out
    }
}

fn sanitize(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn help_text(name: &str) -> &'static str {
    match name {
        "moderation_reports_total" => "Total reports by status",
        "moderation_reports_pending" => "Reports currently open or under review",
        "moderation_actions_total" => "Governance actions by action and target type",
        "moderation_auto_rules_triggered_total" => {
            "Auto-moderation rule triggers by rule type and action"
        }
        "moderation_sanctions_active" => "Active sanctions by type",
        "moderation_appeals_total" => "Appeals by status",
        "moderation_appeals_pending" => "Appeals currently pending",
        "moderation_websocket_events_total" => "Realtime moderation events published by type",
        _ => "Governance metric",
    }
}

#[cfg(test)]
mod tests {
    use super::MetricsRegistry;

    #[test]
    fn renders_prometheus_text_format() {
        let registry = MetricsRegistry::new();
        registry.inc("moderation_reports_total", &[("status", "open")]);
        registry.inc("moderation_reports_total", &[("status", "open")]);
        registry.inc("moderation_reports_total", &[("status", "resolved")]);
        registry.observe_review_duration(30.0);
        let text = registry.render();
        assert!(text.contains("moderation_reports_total{status=\"open\"} 2"));
        assert!(text.contains("moderation_reports_total{status=\"resolved\"} 1"));
        assert!(text.contains("moderation_review_duration_seconds_bucket{le=\"60\"} 1"));
    }
}
