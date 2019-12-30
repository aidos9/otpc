use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Item {
    pub label: String,
    pub secret: String,
    pub digits: u8,
    pub split_time: u32,
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(
            f,
            "({}, {}, {}, {})",
            self.label, self.secret, self.digits, self.split_time
        );
    }
}
