use serde::{Deserialize, Serialize, ser::SerializeSeq};
use ts_rs::TS;

/// Jupyter Notebook format (nbformat 4.x)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Notebook {
    pub nbformat: u32,
    pub nbformat_minor: u32,
    pub metadata: NotebookMetadata,
    pub cells: Vec<NotebookCell>,
}

/// Notebook metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct NotebookMetadata {
    #[serde(default)]
    pub kernelspec: Option<Kernelspec>,
    #[serde(default)]
    pub language_info: Option<LanguageInfo>,
    #[serde(flatten)]
    #[ts(skip)]
    pub extra: serde_json::Value,
}

/// Kernelspec information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Kernelspec {
    pub display_name: String,
    pub language: String,
    pub name: String,
}

/// Language information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
}

/// Notebook cell
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct NotebookCell {
    pub cell_type: CellType,
    #[serde(deserialize_with = "deserialize_source")]
    #[serde(serialize_with = "serialize_source")]
    pub source: String,
    #[serde(default)]
    #[ts(skip)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub outputs: Vec<CellOutput>,
    #[serde(default)]
    pub execution_count: Option<u32>,
}

/// Cell type
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
#[serde(rename_all = "snake_case")]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

/// Cell output
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
#[serde(tag = "output_type")]
#[serde(rename_all = "snake_case")]
pub enum CellOutput {
    ExecuteResult {
        execution_count: Option<u32>,
        data: OutputData,
        #[ts(skip)]
        metadata: serde_json::Value,
    },
    DisplayData {
        data: OutputData,
        #[ts(skip)]
        metadata: serde_json::Value,
    },
    Stream {
        name: String,
        #[serde(deserialize_with = "deserialize_source")]
        text: String,
    },
    Error {
        ename: String,
        evalue: String,
        traceback: Vec<String>,
    },
}

/// Output data (can contain multiple MIME types)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct OutputData {
    #[serde(flatten)]
    #[ts(skip)]
    pub data: serde_json::Value,
}

// Helper functions to normalize cell source (string or array of strings)
fn deserialize_source<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct SourceVisitor;

    impl<'de> Visitor<'de> for SourceVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an array of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut result = String::new();
            while let Some(item) = seq.next_element::<String>()? {
                result.push_str(&item);
            }
            Ok(result)
        }
    }

    deserializer.deserialize_any(SourceVisitor)
}

fn serialize_source<S>(source: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Jupyter notebooks typically store source as an array of strings
    // where each string is a line. We'll split by newlines, keeping the newline character.
    let lines: Vec<String> = source.split_inclusive('\n').map(|s| s.to_string()).collect();
    let mut seq = serializer.serialize_seq(Some(lines.len()))?;
    for line in lines {
        seq.serialize_element(&line)?;
    }
    seq.end()
}

impl Default for NotebookMetadata {
    fn default() -> Self {
        NotebookMetadata {
            kernelspec: Some(Kernelspec {
                display_name: "Julia".to_string(),
                language: "julia".to_string(),
                name: "julia".to_string(),
            }),
            language_info: Some(LanguageInfo {
                name: "julia".to_string(),
                version: "1.0".to_string(),
            }),
            extra: serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserialize_source_from_array() {
        // Test deserializing source from array format (standard Jupyter)
        let json = r#"{
            "cell_type": "code",
            "source": [
                "line 1\n",
                "line 2\n",
                "line 3"
            ],
            "metadata": {},
            "outputs": [],
            "execution_count": null
        }"#;

        let cell: NotebookCell = serde_json::from_str(json).unwrap();
        println!("Test deserialize array: source = {:?}", cell.source);
        println!("  Length: {}", cell.source.len());
        println!("  Newline count: {}", cell.source.matches('\n').count());
        
        assert_eq!(cell.source, "line 1\nline 2\nline 3");
        assert_eq!(cell.source.matches('\n').count(), 2);
    }

    #[test]
    fn test_deserialize_source_from_string() {
        // Test deserializing source from string format
        let json = r#"{
            "cell_type": "code",
            "source": "line 1\nline 2\nline 3",
            "metadata": {},
            "outputs": [],
            "execution_count": null
        }"#;

        let cell: NotebookCell = serde_json::from_str(json).unwrap();
        println!("Test deserialize string: source = {:?}", cell.source);
        println!("  Length: {}", cell.source.len());
        println!("  Newline count: {}", cell.source.matches('\n').count());
        
        assert_eq!(cell.source, "line 1\nline 2\nline 3");
        assert_eq!(cell.source.matches('\n').count(), 2);
    }

    #[test]
    fn test_serialize_source_to_array() {
        // Test serializing source to array format
        let cell = NotebookCell {
            cell_type: CellType::Code,
            source: "line 1\nline 2\nline 3".to_string(),
            metadata: serde_json::json!({}),
            outputs: vec![],
            execution_count: None,
        };

        let json = serde_json::to_string_pretty(&cell).unwrap();
        println!("Test serialize: json = {}", json);
        
        // Check that it serialized as an array
        assert!(json.contains(r#""source":"#) || json.contains(r#""source" :"#));
        assert!(json.contains("line 1\\n"));
        assert!(json.contains("line 2\\n"));
    }

    #[test]
    fn test_roundtrip() {
        // Test that deserialize -> serialize -> deserialize works correctly
        let original_json = r##"{"cell_type": "markdown", "source": ["# Heading\n", "\n", "Some text\n"], "metadata": {}, "outputs": [], "execution_count": null}"##;

        let cell1: NotebookCell = serde_json::from_str(original_json).unwrap();
        println!("Roundtrip test - after deserialize: {:?}", cell1.source);
        println!("  Newlines: {}", cell1.source.matches('\n').count());
        
        let json = serde_json::to_string(&cell1).unwrap();
        println!("Roundtrip test - JSON: {}", json);
        
        let cell2: NotebookCell = serde_json::from_str(&json).unwrap();
        println!("Roundtrip test - after re-deserialize: {:?}", cell2.source);
        println!("  Newlines: {}", cell2.source.matches('\n').count());
        
        assert_eq!(cell1.source, cell2.source);
        assert_eq!(cell1.source, "# Heading\n\nSome text\n");
        assert_eq!(cell1.source.matches('\n').count(), 3);
    }

    #[test]
    fn test_serialize_corrupted_source_without_newlines() {
        // Test what happens when we serialize a source string that has no newlines
        // (simulating a corrupted notebook)
        let cell = NotebookCell {
            cell_type: CellType::Code,
            source: "line1line2line3".to_string(), // No newlines!
            metadata: serde_json::json!({}),
            outputs: vec![],
            execution_count: None,
        };

        let json = serde_json::to_string(&cell).unwrap();
        println!("Corrupted source serialized: {}", json);
        
        // Should serialize as a single-element array
        assert!(json.contains(r#""source":["line1line2line3"]"#));
        
        // Deserialize and check
        let cell2: NotebookCell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell2.source, "line1line2line3");
    }
}
