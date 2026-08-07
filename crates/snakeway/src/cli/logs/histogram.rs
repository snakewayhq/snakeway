#[derive(Clone)]
pub(crate) struct Histogram {
    buckets: &'static [u64],
    counts: Vec<u64>,
}

impl Histogram {
    pub(crate) fn new(buckets: &'static [u64]) -> Self {
        Self {
            buckets,
            counts: vec![0; buckets.len() + 1], // +∞ bucket
        }
    }

    pub(crate) fn record(&mut self, value: u64) {
        for (i, b) in self.buckets.iter().enumerate() {
            if value <= *b {
                self.counts[i] += 1;
                return;
            }
        }
        self.counts[self.buckets.len()] += 1;
    }

    pub(crate) fn snapshot(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();

        for (i, c) in self.counts.iter().enumerate() {
            let label = if i == 0 {
                format!("0–{}ms", self.buckets[0])
            } else if i < self.buckets.len() {
                format!("{}–{}ms", self.buckets[i - 1] + 1, self.buckets[i])
            } else {
                format!(">{}ms", self.buckets[self.buckets.len() - 1])
            };

            out.push((label, *c));
        }

        out
    }

    pub(crate) fn numeric_buckets(&self) -> Vec<(u64, u64)> {
        let mut out = Vec::new();

        for (i, count) in self.counts.iter().enumerate() {
            let upper = if i < self.buckets.len() {
                self.buckets[i]
            } else {
                u64::MAX // overflow bucket
            };
            out.push((upper, *count));
        }

        out
    }
}

pub(crate) fn percentile_from_histogram(buckets: &[(u64, u64)], total: u64, pct: f64) -> u64 {
    if total == 0 {
        return 0;
    }

    let target = (total as f64 * pct).ceil() as u64;
    let mut running = 0;

    for (upper, count) in buckets {
        running += *count;
        if running >= target {
            if *upper == u64::MAX {
                // "greater than last real bucket"
                return buckets
                    .iter()
                    .rev()
                    .find(|(u, _)| *u != u64::MAX)
                    .map(|(u, _)| u.saturating_add(1))
                    .unwrap_or(0);
            }
            return *upper;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const BUCKETS: &[u64] = &[10, 100];

    #[test]
    fn should_bucket_values_inclusively_at_boundaries() {
        // Arrange
        let mut histogram = Histogram::new(BUCKETS);
        histogram.record(10);
        histogram.record(11);
        histogram.record(100);
        histogram.record(101);

        // Act
        let snapshot = histogram.snapshot();

        // Assert
        assert_eq!(
            snapshot,
            vec![
                ("0–10ms".to_string(), 1),
                ("11–100ms".to_string(), 2),
                (">100ms".to_string(), 1),
            ]
        );
    }

    #[test]
    fn should_expose_numeric_buckets_with_overflow_upper_bound() {
        // Arrange
        let mut histogram = Histogram::new(BUCKETS);
        histogram.record(1);
        histogram.record(50);
        histogram.record(500);

        // Act
        let buckets = histogram.numeric_buckets();

        // Assert
        assert_eq!(buckets, vec![(10, 1), (100, 1), (u64::MAX, 1)]);
    }

    #[test]
    fn should_return_bucket_upper_bound_at_percentile() {
        // Arrange
        let buckets = vec![(10, 5), (100, 5), (u64::MAX, 0)];

        // Act
        let median = percentile_from_histogram(&buckets, 10, 0.5);

        // Assert
        assert_eq!(median, 10);
        assert_eq!(percentile_from_histogram(&buckets, 10, 0.95), 100);
    }

    #[test]
    fn should_report_beyond_last_real_bucket_for_overflow_percentile() {
        // Arrange
        let buckets = vec![(10, 1), (100, 0), (u64::MAX, 9)];

        // Act
        let p99 = percentile_from_histogram(&buckets, 10, 0.99);

        // Assert
        assert_eq!(
            p99, 101,
            "overflow percentile is one past the last real bucket"
        );
    }

    #[test]
    fn should_return_zero_for_an_empty_histogram() {
        // Arrange
        let buckets = vec![(10, 0), (u64::MAX, 0)];

        // Act
        let p95 = percentile_from_histogram(&buckets, 0, 0.95);

        // Assert
        assert_eq!(p95, 0);
    }
}
