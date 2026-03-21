use dashmap::DashMap;

#[derive(Default)]
pub(crate) struct Http01Registry {
    /// key: token, value: keyAuthorization
    tokens: DashMap<String, String>,
}

impl Http01Registry {
    pub(crate) fn put(&self, token: String, key_authorization: String) {
        self.tokens.insert(token, key_authorization);
    }

    pub(crate) fn get(&self, token: &str) -> Option<String> {
        self.tokens.get(token).map(|v| v.value().clone())
    }

    pub(crate) fn remove(&self, token: &str) {
        self.tokens.remove(token);
    }
}
