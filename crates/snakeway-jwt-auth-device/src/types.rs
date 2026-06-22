use serde::Deserialize;

pub(crate) struct ValidatedToken {
    pub(crate) claims: JwtClaims,
}

#[derive(Deserialize)]
pub(crate) struct JwtHeader {
    pub(crate) alg: String,
    #[allow(dead_code)]
    typ: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct JwtClaims {
    #[serde(default)]
    pub(crate) iss: Option<String>,

    #[serde(default)]
    pub(crate) aud: Option<Audience>,

    #[serde(default)]
    sub: Option<String>,

    #[serde(default)]
    pub(crate) exp: Option<u64>,

    #[serde(default)]
    pub(crate) nbf: Option<u64>,

    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl JwtClaims {
    pub(crate) fn get_claim(&self, name: &str) -> Option<String> {
        match name {
            "sub" => self.sub.clone(),
            "iss" => self.iss.clone(),
            other => self.extra.get(other).and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            }),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    pub(crate) fn contains(&self, expected: &str) -> bool {
        match self {
            Audience::Single(s) => s == expected,
            Audience::Multiple(v) => v.iter().any(|s| s == expected),
        }
    }
}
