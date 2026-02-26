use dashmap::DashMap;

#[derive(Default)]
pub struct Http01Registry {
    /// key: token, value: keyAuthorization
    tokens: DashMap<String, String>,
}

impl Http01Registry {
    pub fn put(&self, token: String, key_authorization: String) {
        self.tokens.insert(token, key_authorization);
    }

    pub fn get(&self, token: &str) -> Option<String> {
        self.tokens.get(token).map(|v| v.value().clone())
    }

    pub fn remove(&self, token: &str) {
        self.tokens.remove(token);
    }
}
