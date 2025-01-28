use serde::Deserialize;

pub fn deserialize_default_true<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|b| b.unwrap_or(true))
}
