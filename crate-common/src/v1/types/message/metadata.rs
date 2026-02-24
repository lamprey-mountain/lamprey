use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// arbitrary key-value metadata included for a message.
///
/// - max 8 keys
/// - max 32 chars per key
/// - max 1024 chars per value
/// - max 2048 chars across all values
///
/// included in interaction. only visible to user who sent it (and the owner if its a bot).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageMetadata(pub HashMap<String, String>);

impl MessageMetadata {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }
}

impl FromIterator<(String, String)> for MessageMetadata {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(feature = "validator")]
mod v {
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    use super::MessageMetadata;

    impl Validate for MessageMetadata {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            let mut total_value_len = 0;

            // validate number of keys (max 8)
            if !self.0.validate_length(None, Some(8), None) {
                let mut err = ValidationError::new("length");
                err.add_param("max".into(), &json!(8));
                err.add_param("actual".into(), &(self.0.len() as i64));
                errors.add("data", err);
            }

            for (key, value) in self.0.iter() {
                // validate key length
                if !key.validate_length(Some(1), Some(32), None) {
                    let mut err = ValidationError::new("key_length");
                    err.add_param("max".into(), &json!(32));
                    err.add_param("min".into(), &json!(1));
                    err.add_param("actual".into(), &(key.len() as i64));
                    errors.add("key", err);
                }

                // validate value length
                if !value.validate_length(None, Some(1024), None) {
                    let mut err = ValidationError::new("value_length");
                    err.add_param("max".into(), &json!(1024));
                    err.add_param("actual".into(), &(value.len() as i64));
                    errors.add("value", err);
                }

                total_value_len += value.len();
            }

            // validate total value length
            if total_value_len > 2048 {
                let mut err = ValidationError::new("total_value_length");
                err.add_param("max".into(), &json!(2048));
                err.add_param("actual".into(), &(total_value_len as i64));
                errors.add("total_value", err);
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }
}
