use crate::policy::{Frequency, Limit, LimitView};

use std::collections::HashMap;

pub struct AllocStore {
    values: HashMap<String, Frequency>,
}

impl AllocStore {
    pub fn new(limits: Vec<Limit>) -> Self {
        let mut values = HashMap::new();
        for limit in limits {
            values.insert(limit.bucket, limit.freq);
        }
        Self { values }
    }

    pub fn all(&self) -> Vec<LimitView> {
        self.values
            .iter()
            .map(|(k, v)| LimitView {
                bucket: k.as_ref(),
                freq: v.clone(),
            })
            .collect()
    }

    pub fn bucket_config(&self, name: &str) -> Option<LimitView> {
        self.values.get_key_value(name).map(|(k, v)| LimitView {
            bucket: k.as_ref(),
            freq: v.clone(),
        })
    }
}
