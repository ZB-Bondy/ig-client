use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub trait Serializable: Serialize + for<'de> Deserialize<'de> {}

impl<T> Serializable for T where T: Serialize + for<'de> Deserialize<'de> {}

pub struct Serializer;

impl Serializer {
    pub fn json<T: Serializable>(value: &T) -> Result<String> {
        serde_json::to_string(value).context("Failed to serialize to JSON")
    }

    pub fn from_json<T: Serializable>(json: &str) -> Result<T> {
        serde_json::from_str(json).context("Failed to deserialize from JSON")
    }

    pub fn json_pretty<T: Serializable>(value: &T) -> Result<String> {
        serde_json::to_string_pretty(value).context("Failed to serialize to pretty JSON")
    }
}

#[cfg(test)]
mod tests_serializer {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        id: u32,
        name: String,
    }

    #[test]
    fn test_serialization_deserialization() {
        let test_struct = TestStruct {
            id: 1,
            name: "Test".to_string(),
        };

        let json = Serializer::json(&test_struct).unwrap();
        let deserialized: TestStruct = Serializer::from_json(&json).unwrap();

        assert_eq!(test_struct, deserialized);
    }

    #[test]
    fn test_pretty_json() {
        let test_struct = TestStruct {
            id: 1,
            name: "Test".to_string(),
        };

        let json = Serializer::json_pretty(&test_struct).unwrap();

        assert!(json.contains("\n"));
        assert!(json.contains("  "));
    }

    #[test]
    fn test_invalid_json() {
        let invalid_json = r#"{"id": 1, "name": "Test""#;
        let result: Result<TestStruct> = Serializer::from_json(invalid_json);
        assert!(result.is_err());
    }
}
