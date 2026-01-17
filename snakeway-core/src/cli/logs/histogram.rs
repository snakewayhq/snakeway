#[derive(Clone)]
pub struct Histogram {
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
        *self.counts.last_mut().unwrap() += 1;
    }

    pub(crate) fn snapshot(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();

        for (i, c) in self.counts.iter().enumerate() {
            let label = if i == 0 {
                format!("0–{}ms", self.buckets[0])
            } else if i < self.buckets.len() {
                format!("{}–{}ms", self.buckets[i - 1] + 1, self.buckets[i])
            } else {
                format!(">{}ms", self.buckets.last().unwrap())
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

pub fn percentile_from_histogram(buckets: &[(u64, u64)], total: u64, pct: f64) -> u64 {
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
