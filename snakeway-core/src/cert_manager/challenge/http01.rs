use dashmap::DashMap;

pub struct Http01Registry {
    tokens: DashMap<String, String>,
}
