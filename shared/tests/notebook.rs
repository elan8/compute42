
use shared::frontend::notebook::{NotebookCell, CellType};
use serde_json::json;

#[test]
fn test_notebook_cell_source_roundtrip() {
    let original_source = "line1\nline2\nline3";
    let cell = NotebookCell {
        cell_type: CellType::Code,
        source: original_source.to_string(),
        metadata: json!({}),
        outputs: vec![],
        execution_count: None,
    };

    // Serialize
    let json = serde_json::to_string(&cell).unwrap();
    println!("Serialized: {}", json);

    // Deserialize
    let deserialized: NotebookCell = serde_json::from_str(&json).unwrap();
    
    // Assert
    assert_eq!(deserialized.source, original_source, "Source should be preserved after roundtrip");
}
