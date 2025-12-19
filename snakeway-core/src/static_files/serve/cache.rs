use crate::config::StaticCachePolicy;
use http::{HeaderMap, HeaderValue};

pub(crate) fn apply_cache_headers(headers: &mut HeaderMap, policy: &StaticCachePolicy) {
    let mut value = String::new();

    if policy.public {
        value.push_str("public");
    } else {
        value.push_str("private");
    }

    value.push_str(&format!(", max-age={}", policy.max_age));

    if policy.immutable {
        value.push_str(", immutable");
    }

    headers.insert(
        http::header::CACHE_CONTROL,
        HeaderValue::from_str(&value).unwrap(),
    );
}
