use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;
use std::collections::HashMap;
use indexmap::IndexMap;

static TYPE_PREFIX_REGEX: OnceLock<Regex> = OnceLock::new();

/// Get the compiled regex for matching type prefixes
fn get_type_prefix_regex() -> &'static Regex {
    TYPE_PREFIX_REGEX.get_or_init(|| {
        // Match patterns like "Float32[", "Int64[", "UInt8[" at the start of a string
        Regex::new(r"^[A-Za-z0-9]+\[").unwrap()
    })
}

/// Clean array string representation by removing type prefixes
/// 
/// Examples:
/// - "Float32[1.0, 2.0, 3.0]" -> "[1.0, 2.0, 3.0]"
/// - "Int64[1 2 3; 4 5 6]" -> "[1 2 3; 4 5 6]"
pub fn clean_array_string(value: &str) -> String {
    let regex = get_type_prefix_regex();
    regex.replace(value, "[").to_string()
}

/// Calculate dimensions string from array value
/// 
/// For 1D arrays: returns just the count (e.g., "250")
/// For 2D matrices: returns "rows × cols" (e.g., "4 × 250")
pub fn calculate_dimensions(value: &str) -> Option<String> {
    let cleaned = clean_array_string(value);
    
    // Check if it's a 1D array (no semicolons)
    if cleaned.starts_with('[') && cleaned.ends_with(']') && !cleaned.contains(';') {
        // Count elements by splitting on commas
        let inner = cleaned.trim_start_matches('[').trim_end_matches(']');
        let count = inner.split(',').filter(|s| !s.trim().is_empty()).count();
        return Some(count.to_string());
    }
    
    // Check if it's a 2D matrix (has semicolons)
    if cleaned.contains(';') {
        let inner = cleaned.trim_start_matches('[').trim_end_matches(']');
        let rows: Vec<&str> = inner.split(';').collect();
        let num_rows = rows.len();
        
        if let Some(first_row) = rows.first() {
            let num_cols = first_row.split_whitespace().filter(|s| !s.is_empty()).count();
            return Some(format!("{} × {}", num_rows, num_cols));
        }
    }
    
    None
}

/// Process variable data from Julia, cleaning values and adding computed fields
/// 
/// This function:
/// - Cleans array string representations (removes type prefixes)
/// - Calculates and adds dimensions for arrays
/// - Parses DataFrames into structured table data
/// - Ensures consistent data format for the frontend
pub fn process_variable_data(mut var_data: Value) -> Value {
    if let Some(obj) = var_data.as_object_mut() {
        // Check if this variable is an array
        let is_array = obj.get("is_array")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Check if this variable is a DataFrame
        let is_dataframe = obj.get("is_dataframe")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        if is_dataframe {
            // Parse DataFrame value if present
            if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                if let Some(parsed) = parse_dataframe(value_str, obj.get("column_names")) {
                    obj.insert("parsed_data".to_string(), parsed);
                }
            }
        } else if is_array {
            // Clean the value string if present
            if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                let cleaned = clean_array_string(value_str);
                obj.insert("value".to_string(), Value::String(cleaned.clone()));
                
                // Calculate and add dimensions if not already present
                if !obj.contains_key("dimensions") {
                    if let Some(dims) = calculate_dimensions(&cleaned) {
                        obj.insert("dimensions".to_string(), Value::String(dims));
                    }
                }
            }
            
            // Clean the summary string if present
            if let Some(summary_str) = obj.get("summary").and_then(|v| v.as_str()) {
                let cleaned = clean_array_string(summary_str);
                obj.insert("summary".to_string(), Value::String(cleaned));
            }
        }
    }
    
    var_data
}

/// Parse DataFrame string representation into structured table data
/// 
/// This function parses Julia DataFrame output format generically, without hardcoded column names.
/// It works for any DataFrame by analyzing the structure of the output.
fn parse_dataframe(value_str: &str, column_names_value: Option<&Value>) -> Option<Value> {
    // Extract column names from provided array or parse from DataFrame header
    let column_names: Vec<String> = if let Some(names_val) = column_names_value {
        if let Some(names_array) = names_val.as_array() {
            names_array.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    } else {
        // Parse column names from DataFrame header row generically
        // Julia DataFrames always have this structure:
        //  Row │ col1  col2  col3  ...
        // We need to find the line that contains "Row │" and extract column names
        let mut found_names = Vec::new();
        for line in value_str.lines() {
            // Look for the header line: contains "Row │" but not the separator "─"
            if line.contains("Row │") && !line.contains("─") {
                // Split by │ separator and get column headers
                let parts: Vec<&str> = line.split('│').collect();
                if parts.len() >= 2 {
                    // Column names are in the part after the first │
                    let col_part = parts[1..].join("│");
                    found_names = col_part.split_whitespace()
                        .filter(|s| !s.is_empty()) // Remove empty strings
                        .map(|s| s.to_string())
                        .collect();
                }
                break;
            }
        }
        found_names
    };
    
    if column_names.is_empty() {
        println!("DataFrame parsing: No column names found in header");
        return None;
    }
    
    println!("DataFrame parsing: Found {} columns: {:?}", column_names.len(), column_names);
    
    // Parse DataFrame rows from string representation
    // Julia DataFrames have this structure:
    // N×M DataFrame
    //  Row │ col1  col2  col3  ...
    //      │ Type1 Type2 Type3 ...
    // ─────┼──────────────────────
    //    1 │ val1  val2  val3  ...
    //    2 │ val4  val5  val6  ...
    
    let mut rows: Vec<serde_json::Map<String, Value>> = Vec::new();
    let mut in_data_section = false;
    
    for line in value_str.lines() {
        // Check if we've reached the data section (after the separator line)
        if line.contains("─────") {
            in_data_section = true;
            continue;
        }
        
        // Skip if not in data section
        if !in_data_section {
            continue;
        }
        
        // Parse data rows - look for lines with │ that contain data (not headers)
        if line.contains("│") && !line.contains("Row │") && !line.contains("─") && line.trim().len() > 5 {
            // Split by │ separator
            let parts: Vec<&str> = line.split('│').collect();
            if parts.len() >= 2 {
                // Skip the row number (first part), get the data values
                let values_str = parts[1..].join("│");
                let values: Vec<&str> = values_str.split_whitespace().collect();
                
                // Create row object with column names
                let mut row = serde_json::Map::new();
                for (i, col_name) in column_names.iter().enumerate() {
                    if let Some(value) = values.get(i) {
                        row.insert(col_name.clone(), Value::String(value.to_string()));
                    } else {
                        // Handle missing values (when there are fewer values than columns)
                        row.insert(col_name.clone(), Value::String("".to_string()));
                    }
                }
                
                if !row.is_empty() {
                    rows.push(row);
                }
            }
        }
    }
    
    println!("DataFrame parsing: Successfully parsed {} rows", rows.len());
    
    if rows.is_empty() {
        println!("DataFrame parsing: No data rows found");
        None
    } else {
        Some(Value::Array(
            rows.into_iter().map(Value::Object).collect()
        ))
    }
}

/// Check if a variable should be filtered out (hidden from the user)
/// 
/// This function implements the filtering logic for internal variables that should not
/// be displayed in the variable viewer. 
pub fn should_filter_variable(name: &str, type_name: &str) -> bool {
    // Filter out functions and modules - only show data variables
    let is_function = type_name.contains("Function") 
        || type_name.starts_with("typeof(")
        || name.starts_with("#");
    
    let is_module = type_name.contains("Module");
    
    // Filter out internal JuliaJunction variables
    let is_internal_variable = name.starts_with("C42_") 
        || name.starts_with("c42_")
        || name.starts_with("_")
        || name == "DEBUGGER_AVAILABLE"
        || name == "Compute42Display"
        || type_name.contains("Compute42Display");
    
    is_function || is_module || is_internal_variable
}

/// Filter a HashMap of variables, removing internal ones that shouldn't be shown to users
/// 
/// This function applies the filtering logic to a collection of variables,
/// returning only the variables that should be displayed in the variable viewer.
pub fn filter_variables<V>(variables: HashMap<String, V>) -> HashMap<String, V> 
where
    V: Clone,
{
    // For now, we can't filter based on type information since we don't have access to it
    // This function signature is kept for future extensibility
    // The actual filtering should be done at the source where type information is available
    variables
}

/// Filter variables from a JSON object, removing internal ones
/// 
/// This function processes a JSON object containing variable data and filters out
/// internal variables based on their names and type information.
pub fn filter_variables_from_json(variables: Value) -> Value {
    if let Some(map) = variables.as_object() {
        let filtered: serde_json::Map<String, Value> = map
            .iter()
            .filter(|(name, var_data)| {
                // Extract type information from the variable data
                let type_name = var_data
                    .get("type")
                    .or_else(|| var_data.get("var_type").and_then(|vt| vt.get("name")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                
                // Apply filtering logic
                !should_filter_variable(name, type_name)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        
        Value::Object(filtered)
    } else {
        variables
    }
}

/// Process a map/object of variables, cleaning each one and filtering out internal variables
/// Variables are sorted alphabetically by name (case-insensitive, A-Z)
pub fn process_variables_map(variables: Value) -> Value {
    if let Some(map) = variables.as_object() {
        // Collect and sort variables case-insensitively
        // First collect into a Vec, then sort by lowercase key, then convert directly to serde_json::Map
        let mut processed_vec: Vec<(String, Value)> = map
            .iter()
            .filter(|(name, var_data)| {
                // Extract type information from the variable data
                let type_name = var_data
                    .get("type")
                    .or_else(|| var_data.get("var_type").and_then(|vt| vt.get("name")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                
                // Apply filtering logic - only keep variables that should not be filtered
                !should_filter_variable(name, type_name)
            })
            .map(|(key, value)| {
                (key.clone(), process_variable_data(value.clone()))
            })
            .collect();
        
        // Sort case-insensitively: compare lowercase versions but keep original keys
        processed_vec.sort_by(|a, b| {
            a.0.to_lowercase().cmp(&b.0.to_lowercase())
                .then_with(|| a.0.cmp(&b.0)) // If lowercase equal, use case-sensitive as tiebreaker
        });
        
        // Log first few variable names after sorting (commented out for performance)
        let _first_few: Vec<String> = processed_vec.iter().take(10).map(|(k, _)| k.clone()).collect();
        //log::debug!("process_variables_map: First 10 variable names after sorting: {:?}", _first_few);
        
        // Build IndexMap directly to preserve insertion order (required for alphabetical sorting)
        // serde_json::Map with preserve_order uses IndexMap internally, but we'll use it explicitly
        // to ensure order is maintained throughout serialization
        let index_map: IndexMap<String, Value> = processed_vec.into_iter().collect();
        
        // Verify order is preserved in the IndexMap (commented out for performance)
        let _map_keys: Vec<String> = index_map.keys().take(10).cloned().collect();
        //log::debug!("process_variables_map: First 10 keys in IndexMap: {:?}", _map_keys);
        
        // Convert IndexMap to serde_json::Map (preserves order when preserve_order feature is enabled)
        let json_map: serde_json::Map<String, Value> = index_map.into_iter().collect();
        let _final_keys: Vec<String> = json_map.keys().take(10).cloned().collect();
        //log::debug!("process_variables_map: First 10 keys in serde_json::Map after conversion: {:?}", _final_keys);
        
        Value::Object(json_map)
    } else {
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_clean_array_string() {
        assert_eq!(clean_array_string("Float32[1.0, 2.0, 3.0]"), "[1.0, 2.0, 3.0]");
        assert_eq!(clean_array_string("Int64[1 2 3; 4 5 6]"), "[1 2 3; 4 5 6]");
        assert_eq!(clean_array_string("[1, 2, 3]"), "[1, 2, 3]");
        
        // Test the specific case from the terminal output
        let test_value = "Float32[-1.4143807 -1.3227986 -1.341115 -1.1579509 0.34399512 1.0400189 0.54547566 -1.5975448 -0.33371222 -0.68172413 -1.3227986 -1.3777479 0.65537417 1.1865503 -0.62677485 -1.1762673 0.8202219 -0.5168764 -1.5792284 -0.7733062 0.3073623 0.8751711 -1.7074434 -1.5425956 -1.1579509 1.2598158 -1.3594315 1.1132846 -1.4510136 -1.5059627 -0.13223167 0.23409663 1.4979292 -0.75498974 0.39894438 -0.86488825 0.23409663 -0.5351928 1.2414994 -0.022333173 0.17914739 -1.1213181 -0.9381539 0.9301204 1.2781323 0.4172608 -1.2312165 -0.35202864 -0.7733062 -0.5718256 0.19746381 0.3256787 1.0400189 -1.9089239 -0.4069779 0.65537417 0.60042495 0.5271593 0.39894438 -0.6634077 -0.590142 -1.3594315 -1.46933 0.28904587 1.0766517 0.5637921 0.47221002 1.2598158 -0.46192712 0.65537417 -0.97478676 0.17914739 1.882574 -1.7990254 -1.5059627 0.47221002 0.9301204 -0.5535092 0.61874133 1.1499174 -1.0663688 0.4538936 -0.13223167 0.7652727 -1.6341777 1.7360426 0.6370578 0.7652727 -1.2129002 0.47221002 0.5088429 -0.9015211 -0.11391525 0.98506963 1.6078278 -0.60845846 -0.20549732 0.21578021 0.83853835 -0.44361073 0.27072945 -1.5425956 -1.029736 1.1499174 1.3880308 -1.1213181 1.0400189 1.9008904 -0.68172413 -0.38866147 0.9301204 0.78358907 -1.5242792 0.10588173 0.96675324 -0.11391525 -0.60845846 0.23409663 -1.341115 -1.0480524 1.0766517 -0.5535092 -1.2312165 0.61874133 0.5271593 -1.0480524 -1.1213181 -0.97478676 1.5162457 2.5785978 -1.2861658 -0.26044658 0.96675324 0.08756532 -1.4143807 -0.5718256 0.8751711 0.5088429 0.27072945 -0.29707938 0.4172608 0.28904587 0.5271593 -0.5168764 -1.1396345 0.28904587 1.1132846 -1.1579509 -0.9564703 2.193953 -0.9381539 1.3880308 -0.5168764 -0.7916226 0.78358907 1.351398 0.4538936 -1.3960643 0.47221002 0.8202219 -0.97478676 -1.3044822 0.21578021 -0.809939 -0.7916226 -1.2129002 1.2598158 1.2048666 -0.7733062 -0.13223167 -0.35202864 1.0583353 -0.7733062 0.3256787 -1.3777479 -1.4143807 0.23409663 -1.4510136 -0.6634077 0.4538936 0.8202219 -0.48024353 -0.3153958 -1.7440761 -1.1396345 0.8751711 -0.3153958 1.3147651 1.2781323 0.28904587 1.2414994 -1.1762673 1.4796128 0.6736906 -0.68172413 0.49052644 -0.60845846 1.6261442 0.23409663 1.1499174 1.351398 1.4246635 -0.077282414 0.43557718 -0.15054807 -0.5535092 1.2598158 -0.09559883 -1.1213181 0.47221002 -0.7916226 0.9484368 -1.46933 0.4172608 -0.44361073 -0.9015211 -0.5535092 -2.1653538 -0.46192712 -0.22381374 -0.058966003 1.0949681 0.25241306 -0.077282414 0.8934876 -0.20549732 0.83853835 1.4796128 1.0217025 0.83853835 0.85685474 0.25241306 -0.4252943 1.0217025 0.49052644 -0.26044658 -0.13223167 -0.48024353 2.0474217 -1.2678493 -1.0663688 0.4172608 1.0766517 0.60042495 1.3697144 0.38062796 -1.4510136 2.1756365 0.98506963 1.3147651 1.2048666 -0.86488825 -0.5168764 -1.9821895 1.2048666 1.223183 0.6736906 -0.7916226 0.10588173 1.754359 -0.9198375 -1.1396345 -0.88320464 -1.1030016 1.131601 0.5271593 1.2964487 -1.6341777 -0.9198375 1.1132846 -0.99310315 -0.22381374 -1.0663688; -0.5322935 0.8349383 0.63238543 1.0881294 -1.2918668 0.531109 -1.2918668 -0.63356996 0.98685294 -0.025911367 1.0881294 -0.07654958 -1.0893139 -0.9373992 0.8855765 0.68302363 -1.3931432 0.22727971 0.8855765 0.27791792 1.5945115 -1.0386757 0.024726847 -0.4816553 -0.07654958 0.07536506 -0.27910244 1.1894058 0.32855615 -0.1271878 0.68302363 0.32855615 -0.07654958 0.98685294 -1.0386757 1.9996172 -0.68420815 1.4425969 -1.0893139 -1.6463343 -1.9501635 0.48047078 0.024726847 1.240044 0.37919435 -1.5450578 0.48047078 -1.8488871 0.37919435 0.8855765 -0.886761 -0.07654958 -0.58293176 0.9362147 -1.2412286 -1.4944196 -0.8361228 -0.886761 0.531109 0.42983258 -0.07654958 0.42983258 1.0374912 -1.0893139 -0.17782602 0.07536506 0.37919435 0.9362147 0.68302363 -0.17782602 0.024726847 -1.6969725 1.8477026 0.48047078 -0.07654958 -1.342505 -0.5322935 -0.17782602 -1.6969725 0.8349383 0.22727971 -1.0893139 -1.342505 -0.38037887 0.37919435 -0.68420815 -1.2918668 -1.0386757 -0.17782602 -1.1905903 0.37919435 0.1766415 -1.8995253 1.3919586 1.3413204 0.8349383 0.68302363 -1.342505 -1.5450578 0.58174723 0.7843001 0.1766415 1.0374912 0.7843001 -0.43101707 0.58174723 -1.0893139 -0.73484635 -0.07654958 1.1387676 1.1894058 -1.4437814 0.1766415 -1.2412286 0.531109 -1.595696 0.024726847 -1.1905903 0.32855615 1.4425969 0.07536506 -1.7476107 1.1387676 -0.9373992 -0.5322935 0.48047078 -0.025911367 2.0502555 0.8349383 0.32855615 0.73366183 0.07536506 -0.98803747 0.07536506 0.024726847 0.63238543 -1.5450578 -0.9373992 -1.2918668 2.0502555 -1.1399521 -1.342505 -1.4437814 0.9362147 0.7843001 -0.07654958 -0.43101707 0.98685294 0.9362147 -0.07654958 0.22727971 0.7843001 0.73366183 0.48047078 -0.7854846 0.531109 0.73366183 1.1894058 -1.3931432 -0.43101707 -0.07654958 0.68302363 -1.342505 0.32855615 0.27791792 0.32855615 -0.73484635 -0.98803747 0.8855765 0.9362147 1.5438733 0.73366183 0.63238543 -1.6463343 -0.025911367 0.07536506 -0.27910244 -0.025911367 0.68302363 -0.7854846 -1.2918668 1.9996172 1.1894058 0.63238543 1.3413204 -0.73484635 0.68302363 -0.32974064 0.98685294 -1.7476107 1.2906822 0.8855765 0.48047078 0.58174723 1.4932351 -1.4944196 0.73366183 1.4425969 -1.6969725 -1.4437814 1.3919586 1.5945115 -0.98803747 -0.68420815 1.0374912 -0.27910244 0.68302363 -1.3931432 0.07536506 -1.8488871 0.8349383 -1.342505 -0.27910244 -1.342505 0.68302363 0.7843001 0.8855765 -0.8361228 0.73366183 0.58174723 -1.6463343 -0.5322935 -1.7476107 -1.4944196 -0.4816553 -1.4944196 0.1766415 1.7970644 -0.5322935 -1.0893139 -0.58293176 -1.6969725 0.42983258 -0.4816553 0.32855615 -0.22846423 -0.27910244 1.5945115 -0.58293176 -0.32974064 -0.32974064 -1.3931432 -0.63356996 -1.7476107 0.9362147 0.8855765 0.68302363 1.3413204 -0.73484635 -0.43101707 -0.63356996 1.240044 0.48047078 -0.5322935 1.240044 1.1387676 -1.342505 0.024726847 -1.4437814 1.3919586 0.8349383 -0.58293176 0.7843001 0.73366183 -1.0893139 -0.32974064 0.8349383 0.37919435 0.32855615 -0.98803747 0.37919435 -1.7476107 0.73366183; -0.989581 -0.989581 -1.2029263 -1.4162716 0.646066 -0.56289047 1.4994471 -1.0606961 -0.4206603 -0.56289047 -0.56289047 -0.4206603 1.2149867 1.6416773 -1.4873866 -0.13619995 0.1482604 -1.3451564 -0.989581 -0.56289047 -0.7051206 1.4994471 -0.84735084 -0.4206603 -1.1318111 1.9261376 -1.4162716 -0.3495452 -0.4206603 -1.1318111 -0.63400555 -0.20731504 2.0683677 -1.2029263 1.0016415 -0.3495452 1.0016415 0.1482604 1.5705621 0.50383586 0.8594113 -0.56289047 -1.4873866 0.78829616 -0.3495452 1.1438717 -1.6296167 0.646066 -0.56289047 0.50383586 1.3572168 -0.4206603 1.7127923 -0.77623576 0.646066 0.57495093 1.0016415 1.0016415 -1.6296167 -0.4206603 -0.77623576 -1.3451564 -0.84735084 1.3572168 2.0683677 -1.1318111 -0.63400555 0.646066 0.077145316 -0.13619995 -0.13619995 0.78829616 0.0060302266 -0.56289047 -0.84735084 0.8594113 1.0727565 -0.7051206 1.0727565 0.077145316 -0.989581 1.0727565 0.50383586 -0.13619995 -0.77623576 1.2861018 0.78829616 0.57495093 -0.63400555 1.1438717 -0.4206603 -1.0606961 0.57495093 0.1482604 -0.27843013 -0.56289047 -0.4206603 1.0016415 1.3572168 -0.4206603 -0.9184659 -0.77623576 -0.84735084 -0.20731504 2.0683677 -1.9140772 1.0727565 2.1394827 -1.771847 -0.20731504 0.646066 0.646066 -0.7051206 0.9305264 -0.4206603 0.50383586 -0.989581 0.78829616 -1.1318111 -0.77623576 -0.20731504 0.9305264 -1.2029263 1.4994471 1.0016415 -1.1318111 -1.0606961 -0.7051206 -0.27843013 -1.4162716 -0.84735084 -0.989581 1.428332 1.2861018 -0.989581 -0.4206603 0.646066 1.2861018 0.7171811 -0.7051206 1.428332 0.78829616 1.0016415 -1.3451564 -1.4873866 -0.3495452 2.0683677 -0.4917754 -0.4206603 1.9261376 -0.7051206 -0.989581 -0.84735084 -1.0606961 1.428332 -0.27843013 -0.77623576 -0.77623576 1.1438717 1.3572168 -0.9184659 -0.56289047 0.43272075 -0.9184659 -1.0606961 -0.7051206 1.7839074 1.0727565 -1.2029263 -0.27843013 -0.77623576 -0.4206603 -0.77623576 0.9305264 -1.2029263 -0.989581 -0.7051206 -0.989581 -0.3495452 1.428332 0.8594113 -0.4206603 -0.27843013 -1.2029263 -0.20731504 0.50383586 -1.4873866 1.7127923 -0.3495452 0.9305264 0.1482604 -1.5585017 0.0060302266 -0.4206603 -0.06508486 0.646066 -1.2740413 0.29049057 1.0016415 1.2149867 -0.20731504 -0.4917754 0.8594113 1.0016415 -0.27843013 -0.989581 0.0060302266 1.2149867 -1.4873866 0.646066 -0.77623576 0.78829616 -0.77623576 0.57495093 0.0060302266 -1.1318111 -1.2029263 -0.9184659 -0.7051206 -0.3495452 1.1438717 0.8594113 0.646066 1.3572168 1.4994471 0.57495093 -0.7051206 0.646066 1.6416773 1.2861018 2.0683677 0.50383586 -0.63400555 1.9972527 -0.56289047 -0.989581 -0.989581 -0.4917754 2.0683677 -1.1318111 -0.20731504 0.9305264 1.9972527 0.9305264 0.0060302266 -0.4206603 -1.0606961 0.43272075 1.1438717 1.3572168 1.7127923 -0.4206603 0.29049057 -1.6296167 0.0060302266 -0.56289047 1.0016415 -0.3495452 1.0727565 0.29049057 -0.77623576 -1.2740413 -1.4162716 -0.56289047 1.7127923 -0.84735084 0.1482604 -0.63400555 -1.4162716 1.2149867 -0.77623576 0.50383586 -0.77623576; -0.8127074 -0.50096905 -0.9062289 -1.1244458 -0.0021876376 -0.5321429 0.839506 -1.4361842 -0.25157833 -0.99975044 -0.9374027 -1.093272 0.93302745 1.6811996 -0.313926 0.34072456 0.5277676 -1.2491411 -0.50096905 -1.2491411 0.49659374 1.4318088 -1.2491411 -1.0620981 -0.7503597 1.7435472 -1.6855749 -0.3762737 -0.9374027 -1.3114887 -0.12688299 -0.313926 1.6811996 0.5589414 1.1200705 -0.06453531 1.3694612 0.65246296 1.6811996 0.122507714 1.1200705 -0.5633167 -0.50096905 0.122507714 -0.65683824 0.21602923 -0.3762737 -0.06453531 0.060160037 0.122507714 1.0577228 -0.68801206 1.8682426 -0.7503597 0.6212891 0.49659374 0.96420133 1.1824182 -1.1867934 -1.1867934 -0.5944905 -1.3114887 -0.50096905 0.99537516 1.8682426 -0.62566435 -0.8750551 -0.12688299 -0.40744752 -0.3762737 -0.5633167 0.6836368 0.122507714 -0.9062289 -1.0620981 0.24720305 1.6811996 -0.62566435 0.65246296 -0.50096905 -0.9685766 0.6212891 0.30955073 -1.093272 -0.9374027 1.6188519 0.65246296 1.6188519 -1.4985318 1.2447659 -1.1244458 -0.8127074 0.24720305 -0.18923067 -0.5944905 -0.50096905 0.060160037 0.99537516 1.3694612 0.122507714 -0.84388125 -0.62566435 -0.313926 -0.5321429 1.6188519 -0.99975044 0.6836368 1.8058949 -0.9374027 0.30955073 -0.313926 0.49659374 -1.2803149 0.80833215 0.24720305 0.46541992 -0.9062289 1.2447659 -0.62566435 -0.3762737 -0.65683824 0.5589414 -0.3762737 1.3071135 1.6188519 -0.313926 -1.1244458 -0.50096905 -0.9374027 -0.62566435 -0.8750551 -1.0620981 2.6164148 1.3071135 -1.3114887 -0.3762737 0.30955073 1.2447659 0.74598444 -0.06453531 1.3694612 0.6836368 0.80833215 -0.9685766 -0.7503597 -0.8750551 1.8682426 -0.5633167 -0.9374027 1.7435472 -1.1556196 -1.1867934 -1.093272 0.30955073 1.1200705 -0.5633167 -0.9374027 -0.50096905 0.8706798 1.4941566 -1.6232271 -0.8750551 1.0577228 -1.1244458 -0.8750551 -1.0620981 1.2447659 0.99537516 -0.8127074 0.7148106 0.060160037 -0.7503597 -0.3762737 0.24720305 -1.6855749 -1.1244458 -1.1867934 -0.62566435 0.18485539 0.99537516 2.0552857 0.24720305 0.09133387 -1.093272 -0.8750551 1.4318088 -0.8127074 1.3071135 -0.8127074 0.5589414 -0.18923067 -1.5297056 -0.18923067 -0.43862134 -0.28275216 0.80833215 -0.8127074 0.4342461 0.6836368 1.8682426 -0.62566435 -0.5321429 0.5589414 1.0577228 -0.8750551 -1.2491411 0.30955073 0.49659374 -0.62566435 0.4342461 0.49659374 0.5277676 -1.4361842 0.74598444 -0.25157833 -0.68801206 -0.3762737 -1.4361842 -0.62566435 -0.15805683 0.8706798 1.4941566 0.122507714 0.6212891 2.2423286 0.6212891 -0.99975044 0.74598444 1.8058949 0.80833215 1.9929379 -0.0021876376 -0.313926 1.9929379 -0.50096905 -1.0620981 -1.6232271 -0.8127074 2.0552857 -0.99975044 -0.4697952 0.5589414 2.179981 0.9018536 -0.313926 -0.06453531 -1.3738365 -0.25157833 2.0552857 2.2423286 1.4941566 0.59011525 0.122507714 -1.6232271 -0.18923067 -0.50096905 1.4941566 -0.8127074 -0.12688299 0.3718984 -0.7503597 -1.4050103 -0.5633167 -1.5920533 0.99537516 -0.68801206 -0.12688299 -0.5944905 -0.7191859 1.8682426 -1.093272 -0.313926 -0.62566435]";
        let expected = test_value.strip_prefix("Float32").unwrap();
        assert_eq!(clean_array_string(test_value), expected);
    }

    #[test]
    fn test_calculate_dimensions() {
        assert_eq!(calculate_dimensions("[1, 2, 3]"), Some("3".to_string()));
        assert_eq!(calculate_dimensions("[1 2 3; 4 5 6]"), Some("2 × 3".to_string()));
        assert_eq!(calculate_dimensions("Float32[1.0, 2.0]"), Some("2".to_string()));
    }

    #[test]
    fn test_process_variable_data() {
        let input = json!({
            "name": "x",
            "type": "Array{Float32, 1}",
            "is_array": true,
            "value": "Float32[1.0, 2.0, 3.0]",
            "summary": "Float32[1.0, 2.0, 3.0]"
        });
        
        let output = process_variable_data(input);
        
        assert_eq!(output["value"], "[1.0, 2.0, 3.0]");
        assert_eq!(output["summary"], "[1.0, 2.0, 3.0]");
        assert_eq!(output["dimensions"], "3");
    }

    #[test]
    fn test_should_filter_variable() {
        // User variables should not be filtered
        assert!(!should_filter_variable("user_var", "Int64"));
        assert!(!should_filter_variable("my_array", "Array{Int64,1}"));
        assert!(!should_filter_variable("data", "DataFrame"));
        
        // C42 internal variables should be filtered
        assert!(should_filter_variable("C42_INTERNAL", "String"));
        assert!(should_filter_variable("c42_internal_var", "String"));
        assert!(should_filter_variable("DEBUGGER_AVAILABLE", "Bool"));
        assert!(should_filter_variable("Compute42Display", "Module"));
        
        // Julia internal variables should be filtered
        assert!(should_filter_variable("_private_var", "Float64"));
        assert!(should_filter_variable("_julia_internal", "Function"));
        assert!(should_filter_variable("_module_var", "Module"));
        
        // Functions and modules should be filtered
        assert!(should_filter_variable("some_function", "Function"));
        assert!(should_filter_variable("SomeModule", "Module"));
        assert!(should_filter_variable("#1", "Int64")); // Generated function names
        
        // Internal types should be filtered
        assert!(should_filter_variable("display_obj", "Compute42Display"));
    }

    #[test]
    fn test_filter_variables_from_json() {
        let input = json!({
            "user_var": {
                "type": "Int64",
                "value": "42"
            },
            "JJ_PLOT_SERVER": {
                "type": "Nothing",
                "value": "nothing"
            },
            "jj_internal": {
                "type": "String",
                "value": "internal"
            },
            "_private": {
                "type": "Float64",
                "value": "3.14"
            },
            "my_array": {
                "var_type": {
                    "name": "Array{Int64,1}"
                },
                "value": "[1, 2, 3]"
            }
        });
        
        let filtered = filter_variables_from_json(input);
        let filtered_obj = filtered.as_object().unwrap();
        
        // Should contain user variables and JJ_ variables (not filtered)
        assert_eq!(filtered_obj.len(), 4);
        assert!(filtered_obj.contains_key("user_var"));
        assert!(filtered_obj.contains_key("JJ_PLOT_SERVER"));
        assert!(filtered_obj.contains_key("jj_internal"));
        assert!(filtered_obj.contains_key("my_array"));
        
        // Should not contain internal variables (C42_ or _ prefix)
        assert!(!filtered_obj.contains_key("_private"));
    }

    #[test]
    fn test_process_variables_map_with_filtering() {
        let input = json!({
            "user_var": {
                "type": "Int64",
                "value": "42"
            },
            "jj_success1": {
                "type": "Bool",
                "value": "true"
            },
            "jj_success2": {
                "type": "Bool", 
                "value": "false"
            },
            "_private": {
                "type": "Float64",
                "value": "3.14"
            },
            "my_array": {
                "var_type": {
                    "name": "Array{Int64,1}"
                },
                "is_array": true,
                "value": "Int64[1, 2, 3]"
            }
        });
        
        let processed = process_variables_map(input);
        let processed_obj = processed.as_object().unwrap();
        
        // Should contain user variables and jj_ variables (not filtered) after processing and filtering
        assert_eq!(processed_obj.len(), 4);
        assert!(processed_obj.contains_key("user_var"));
        assert!(processed_obj.contains_key("jj_success1"));
        assert!(processed_obj.contains_key("jj_success2"));
        assert!(processed_obj.contains_key("my_array"));
        
        // Should not contain internal variables (C42_ or _ prefix)
        assert!(!processed_obj.contains_key("_private"));
        
        // Check that array processing still works (type prefix should be removed)
        let my_array = processed_obj.get("my_array").unwrap();
        // The value should be cleaned (type prefix removed) by process_variable_data
        assert_eq!(my_array["value"], "[1, 2, 3]");
    }

    #[test]
    fn test_parse_dataframe_generic() {
        // Test generic DataFrame parsing with different column names
        let test_dataframe = r#"3×2 DataFrame
 Row │ name    age
     │ String  Int64
─────┼──────────────
   1 │ Alice   25
   2 │ Bob     30
   3 │ Charlie 35"#;
        
        let result = parse_dataframe(test_dataframe, None);
        assert!(result.is_some());
        
        let parsed = result.unwrap();
        let rows = parsed.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        
        // Check first row
        let first_row = rows[0].as_object().unwrap();
        assert_eq!(first_row.get("name").unwrap().as_str().unwrap(), "Alice");
        assert_eq!(first_row.get("age").unwrap().as_str().unwrap(), "25");
        
        // Check second row
        let second_row = rows[1].as_object().unwrap();
        assert_eq!(second_row.get("name").unwrap().as_str().unwrap(), "Bob");
        assert_eq!(second_row.get("age").unwrap().as_str().unwrap(), "30");
    }

    #[test]
    fn test_clean_execution_complete_result() {
        // Test the specific case from the terminal output - ExecutionComplete with VariableValue
        let test_result = r#"{"value":"Float32[-1.4143807 -1.3227986 -1.341115 -1.1579509 0.34399512 1.0400189 0.54547566 -1.5975448 -0.33371222 -0.68172413 -1.3227986 -1.3777479 0.65537417 1.1865503 -0.62677485 -1.1762673 0.8202219 -0.5168764 -1.5792284 -0.7733062 0.3073623 0.8751711 -1.7074434 -1.5425956 -1.1579509 1.2598158 -1.3594315 1.1132846 -1.4510136 -1.5059627 -0.13223167 0.23409663 1.4979292 -0.75498974 0.39894438 -0.86488825 0.23409663 -0.5351928 1.2414994 -0.022333173 0.17914739 -1.1213181 -0.9381539 0.9301204 1.2781323 0.4172608 -1.2312165 -0.35202864 -0.7733062 -0.5718256 0.19746381 0.3256787 1.0400189 -1.9089239 -0.4069779 0.65537417 0.60042495 0.5271593 0.39894438 -0.6634077 -0.590142 -1.3594315 -1.46933 0.28904587 1.0766517 0.5637921 0.47221002 1.2598158 -0.46192712 0.65537417 -0.97478676 0.17914739 1.882574 -1.7990254 -1.5059627 0.47221002 0.9301204 -0.5535092 0.61874133 1.1499174 -1.0663688 0.4538936 -0.13223167 0.7652727 -1.6341777 1.7360426 0.6370578 0.7652727 -1.2129002 0.47221002 0.5088429 -0.9015211 -0.11391525 0.98506963 1.6078278 -0.60845846 -0.20549732 0.21578021 0.83853835 -0.44361073 0.27072945 -1.5425956 -1.029736 1.1499174 1.3880308 -1.1213181 1.0400189 1.9008904 -0.68172413 -0.38866147 0.9301204 0.78358907 -1.5242792 0.10588173 0.96675324 -0.11391525 -0.60845846 0.23409663 -1.341115 -1.0480524 1.0766517 -0.5535092 -1.2312165 0.61874133 0.5271593 -1.0480524 -1.1213181 -0.97478676 1.5162457 2.5785978 -1.2861658 -0.26044658 0.96675324 0.08756532 -1.4143807 -0.5718256 0.8751711 0.5088429 0.27072945 -0.29707938 0.4172608 0.28904587 0.5271593 -0.5168764 -1.1396345 0.28904587 1.1132846 -1.1579509 -0.9564703 2.193953 -0.9381539 1.3880308 -0.5168764 -0.7916226 0.78358907 1.351398 0.4538936 -1.3960643 0.47221002 0.8202219 -0.97478676 -1.3044822 0.21578021 -0.809939 -0.7916226 -1.2129002 1.2598158 1.2048666 -0.7733062 -0.13223167 -0.35202864 1.0583353 -0.7733062 0.3256787 -1.3777479 -1.4143807 0.23409663 -1.4510136 -0.6634077 0.4538936 0.8202219 -0.48024353 -0.3153958 -1.7440761 -1.1396345 0.8751711 -0.3153958 1.3147651 1.2781323 0.28904587 1.2414994 -1.1762673 1.4796128 0.6736906 -0.68172413 0.49052644 -0.60845846 1.6261442 0.23409663 1.1499174 1.351398 1.4246635 -0.077282414 0.43557718 -0.15054807 -0.5535092 1.2598158 -0.09559883 -1.1213181 0.47221002 -0.7916226 0.9484368 -1.46933 0.4172608 -0.44361073 -0.9015211 -0.5535092 -2.1653538 -0.46192712 -0.22381374 -0.058966003 1.0949681 0.25241306 -0.077282414 0.8934876 -0.20549732 0.83853835 1.4796128 1.0217025 0.83853835 0.85685474 0.25241306 -0.4252943 1.0217025 0.49052644 -0.26044658 -0.13223167 -0.48024353 2.0474217 -1.2678493 -1.0663688 0.4172608 1.0766517 0.60042495 1.3697144 0.38062796 -1.4510136 2.1756365 0.98506963 1.3147651 1.2048666 -0.86488825 -0.5168764 -1.9821895 1.2048666 1.223183 0.6736906 -0.7916226 0.10588173 1.754359 -0.9198375 -1.1396345 -0.88320464 -1.1030016 1.131601 0.5271593 1.2964487 -1.6341777 -0.9198375 1.1132846 -0.99310315 -0.22381374 -1.0663688; -0.5322935 0.8349383 0.63238543 1.0881294 -1.2918668 0.531109 -1.2918668 -0.63356996 0.98685294 -0.025911367 1.0881294 -0.07654958 -1.0893139 -0.9373992 0.8855765 0.68302363 -1.3931432 0.22727971 0.8855765 0.27791792 1.5945115 -1.0386757 0.024726847 -0.4816553 -0.07654958 0.07536506 -0.27910244 1.1894058 0.32855615 -0.1271878 0.68302363 0.32855615 -0.07654958 0.98685294 -1.0386757 1.9996172 -0.68420815 1.4425969 -1.0893139 -1.6463343 -1.9501635 0.48047078 0.024726847 1.240044 0.37919435 -1.5450578 0.48047078 -1.8488871 0.37919435 0.8855765 -0.886761 -0.07654958 -0.58293176 0.9362147 -1.2412286 -1.4944196 -0.8361228 -0.886761 0.531109 0.42983258 -0.07654958 0.42983258 1.0374912 -1.0893139 -0.17782602 0.07536506 0.37919435 0.9362147 0.68302363 -0.17782602 0.024726847 -1.6969725 1.8477026 0.48047078 -0.07654958 -1.342505 -0.5322935 -0.17782602 -1.6969725 0.8349383 0.22727971 -1.0893139 -1.342505 -0.38037887 0.37919435 -0.68420815 -1.2918668 -1.0386757 -0.17782602 -1.1905903 0.37919435 0.1766415 -1.8995253 1.3919586 1.3413204 0.8349383 0.68302363 -1.342505 -1.5450578 0.58174723 0.7843001 0.1766415 1.0374912 0.7843001 -0.43101707 0.58174723 -1.0893139 -0.73484635 -0.07654958 1.1387676 1.1894058 -1.4437814 0.1766415 -1.2412286 0.531109 -1.595696 0.024726847 -1.1905903 0.32855615 1.4425969 0.07536506 -1.7476107 1.1387676 -0.9373992 -0.5322935 0.48047078 -0.025911367 2.0502555 0.8349383 0.32855615 0.73366183 0.07536506 -0.98803747 0.07536506 0.024726847 0.63238543 -1.5450578 -0.9373992 -1.2918668 2.0502555 -1.1399521 -1.342505 -1.4437814 0.9362147 0.7843001 -0.07654958 -0.43101707 0.98685294 0.9362147 -0.07654958 0.22727971 0.7843001 0.73366183 0.48047078 -0.7854846 0.531109 0.73366183 1.1894058 -1.3931432 -0.43101707 -0.07654958 0.68302363 -1.342505 0.32855615 0.27791792 0.32855615 -0.73484635 -0.98803747 0.8855765 0.9362147 1.5438733 0.73366183 0.63238543 -1.6463343 -0.025911367 0.07536506 -0.27910244 -0.025911367 0.68302363 -0.7854846 -1.2918668 1.9996172 1.1894058 0.63238543 1.3413204 -0.73484635 0.68302363 -0.32974064 0.98685294 -1.7476107 1.2906822 0.8855765 0.48047078 0.58174723 1.4932351 -1.4944196 0.73366183 1.4425969 -1.6969725 -1.4437814 1.3919586 1.5945115 -0.98803747 -0.68420815 1.0374912 -0.27910244 0.68302363 -1.3931432 0.07536506 -1.8488871 0.8349383 -1.342505 -0.27910244 -1.342505 0.68302363 0.7843001 0.8855765 -0.8361228 0.73366183 0.58174723 -1.6463343 -0.5322935 -1.7476107 -1.4944196 -0.4816553 -1.4944196 0.1766415 1.7970644 -0.5322935 -1.0893139 -0.58293176 -1.6969725 0.42983258 -0.4816553 0.32855615 -0.22846423 -0.27910244 1.5945115 -0.58293176 -0.32974064 -0.32974064 -1.3931432 -0.63356996 -1.7476107 0.9362147 0.8855765 0.68302363 1.3413204 -0.73484635 -0.43101707 -0.63356996 1.240044 0.48047078 -0.5322935 1.240044 1.1387676 -1.342505 0.024726847 -1.4437814 1.3919586 0.8349383 -0.58293176 0.7843001 0.73366183 -1.0893139 -0.32974064 0.8349383 0.37919435 0.32855615 -0.98803747 0.37919435 -1.7476107 0.73366183; -0.989581 -0.989581 -1.2029263 -1.4162716 0.646066 -0.56289047 1.4994471 -1.0606961 -0.4206603 -0.56289047 -0.56289047 -0.4206603 1.2149867 1.6416773 -1.4873866 -0.13619995 0.1482604 -1.3451564 -0.989581 -0.56289047 -0.7051206 1.4994471 -0.84735084 -0.4206603 -1.1318111 1.9261376 -1.4162716 -0.3495452 -0.4206603 -1.1318111 -0.63400555 -0.20731504 2.0683677 -1.2029263 1.0016415 -0.3495452 1.0016415 0.1482604 1.5705621 0.50383586 0.8594113 -0.56289047 -1.4873866 0.78829616 -0.3495452 1.1438717 -1.6296167 0.646066 -0.56289047 0.50383586 1.3572168 -0.4206603 1.7127923 -0.77623576 0.646066 0.57495093 1.0016415 1.0016415 -1.6296167 -0.4206603 -0.77623576 -1.3451564 -0.84735084 1.3572168 2.0683677 -1.1318111 -0.63400555 0.646066 0.077145316 -0.13619995 -0.13619995 0.78829616 0.0060302266 -0.56289047 -0.84735084 0.8594113 1.0727565 -0.7051206 1.0727565 0.077145316 -0.989581 1.0727565 0.50383586 -0.13619995 -0.77623576 1.2861018 0.78829616 0.57495093 -0.63400555 1.1438717 -0.4206603 -1.0606961 0.57495093 0.1482604 -0.27843013 -0.56289047 -0.4206603 1.0016415 1.3572168 -0.4206603 -0.9184659 -0.77623576 -0.84735084 -0.20731504 2.0683677 -1.9140772 1.0727565 2.1394827 -1.771847 -0.20731504 0.646066 0.646066 -0.7051206 0.9305264 -0.4206603 0.50383586 -0.989581 0.78829616 -1.1318111 -0.77623576 -0.20731504 0.9305264 -1.2029263 1.4994471 1.0016415 -1.1318111 -1.0606961 -0.7051206 -0.27843013 -1.4162716 -0.84735084 -0.989581 1.428332 1.2861018 -0.989581 -0.4206603 0.646066 1.2861018 0.7171811 -0.7051206 1.428332 0.78829616 1.0016415 -1.3451564 -1.4873866 -0.3495452 2.0683677 -0.4917754 -0.4206603 1.9261376 -0.7051206 -0.989581 -0.84735084 -1.0606961 1.428332 -0.27843013 -0.77623576 -0.77623576 1.1438717 1.3572168 -0.9184659 -0.56289047 0.43272075 -0.9184659 -1.0606961 -0.7051206 1.7839074 1.0727565 -1.2029263 -0.27843013 -0.77623576 -0.4206603 -0.77623576 0.9305264 -1.2029263 -0.989581 -0.7051206 -0.989581 -0.3495452 1.428332 0.8594113 -0.4206603 -0.27843013 -1.2029263 -0.20731504 0.50383586 -1.4873866 1.7127923 -0.3495452 0.9305264 0.1482604 -1.5585017 0.0060302266 -0.4206603 -0.06508486 0.646066 -1.2740413 0.29049057 1.0016415 1.2149867 -0.20731504 -0.4917754 0.8594113 1.0016415 -0.27843013 -0.989581 0.0060302266 1.2149867 -1.4873866 0.646066 -0.77623576 0.78829616 -0.77623576 0.57495093 0.0060302266 -1.1318111 -1.2029263 -0.9184659 -0.7051206 -0.3495452 1.1438717 0.8594113 0.646066 1.3572168 1.4994471 0.57495093 -0.7051206 0.646066 1.6416773 1.2861018 2.0683677 0.50383586 -0.63400555 1.9972527 -0.56289047 -0.989581 -0.989581 -0.4917754 2.0683677 -1.1318111 -0.20731504 0.9305264 1.9972527 0.9305264 0.0060302266 -0.4206603 -1.0606961 0.43272075 1.1438717 1.3572168 1.7127923 -0.4206603 0.29049057 -1.6296167 0.0060302266 -0.56289047 1.0016415 -0.3495452 1.0727565 0.29049057 -0.77623576 -1.2740413 -1.4162716 -0.56289047 1.7127923 -0.84735084 0.1482604 -0.63400555 -1.4162716 1.2149867 -0.77623576 0.50383586 -0.77623576; -0.8127074 -0.50096905 -0.9062289 -1.1244458 -0.0021876376 -0.5321429 0.839506 -1.4361842 -0.25157833 -0.99975044 -0.9374027 -1.093272 0.93302745 1.6811996 -0.313926 0.34072456 0.5277676 -1.2491411 -0.50096905 -1.2491411 0.49659374 1.4318088 -1.2491411 -1.0620981 -0.7503597 1.7435472 -1.6855749 -0.3762737 -0.9374027 -1.3114887 -0.12688299 -0.313926 1.6811996 0.5589414 1.1200705 -0.06453531 1.3694612 0.65246296 1.6811996 0.122507714 1.1200705 -0.5633167 -0.50096905 0.122507714 -0.65683824 0.21602923 -0.3762737 -0.06453531 0.060160037 0.122507714 1.0577228 -0.68801206 1.8682426 -0.7503597 0.6212891 0.49659374 0.96420133 1.1824182 -1.1867934 -1.1867934 -0.5944905 -1.3114887 -0.50096905 0.99537516 1.8682426 -0.62566435 -0.8750551 -0.12688299 -0.40744752 -0.3762737 -0.5633167 0.6836368 0.122507714 -0.9062289 -1.0620981 0.24720305 1.6811996 -0.62566435 0.65246296 -0.50096905 -0.9685766 0.6212891 0.30955073 -1.093272 -0.9374027 1.6188519 0.65246296 1.6188519 -1.4985318 1.2447659 -1.1244458 -0.8127074 0.24720305 -0.18923067 -0.5944905 -0.50096905 0.060160037 0.99537516 1.3694612 0.122507714 -0.84388125 -0.62566435 -0.313926 -0.5321429 1.6188519 -0.99975044 0.6836368 1.8058949 -0.9374027 0.30955073 -0.313926 0.49659374 -1.2803149 0.80833215 0.24720305 0.46541992 -0.9062289 1.2447659 -0.62566435 -0.3762737 -0.65683824 0.5589414 -0.3762737 1.3071135 1.6188519 -0.313926 -1.1244458 -0.50096905 -0.9374027 -0.62566435 -0.8750551 -1.0620981 2.6164148 1.3071135 -1.3114887 -0.3762737 0.30955073 1.2447659 0.74598444 -0.06453531 1.3694612 0.6836368 0.80833215 -0.9685766 -0.7503597 -0.8750551 1.8682426 -0.5633167 -0.9374027 1.7435472 -1.1556196 -1.1867934 -1.093272 0.30955073 1.1200705 -0.5633167 -0.9374027 -0.50096905 0.8706798 1.4941566 -1.6232271 -0.8750551 1.0577228 -1.1244458 -0.8750551 -1.0620981 1.2447659 0.99537516 -0.8127074 0.7148106 0.060160037 -0.7503597 -0.3762737 0.24720305 -1.6855749 -1.1244458 -1.1867934 -0.62566435 0.18485539 0.99537516 2.0552857 0.24720305 0.09133387 -1.093272 -0.8750551 1.4318088 -0.8127074 1.3071135 -0.8127074 0.5589414 -0.18923067 -1.5297056 -0.18923067 -0.43862134 -0.28275216 0.80833215 -0.8127074 0.4342461 0.6836368 1.8682426 -0.62566435 -0.5321429 0.5589414 1.0577228 -0.8750551 -1.2491411 0.30955073 0.49659374 -0.62566435 0.4342461 0.49659374 0.5277676 -1.4361842 0.74598444 -0.25157833 -0.68801206 -0.3762737 -1.4361842 -0.62566435 -0.15805683 0.8706798 1.4941566 0.122507714 0.6212891 2.2423286 0.6212891 -0.99975044 0.74598444 1.8058949 0.80833215 1.9929379 -0.0021876376 -0.313926 1.9929379 -0.50096905 -1.0620981 -1.6232271 -0.8127074 2.0552857 -0.99975044 -0.4697952 0.5589414 2.179981 0.9018536 -0.313926 -0.06453531 -1.3738365 -0.25157833 2.0552857 2.2423286 1.4941566 0.59011525 0.122507714 -1.6232271 -0.18923067 -0.50096905 1.4941566 -0.8127074 -0.12688299 0.3718984 -0.7503597 -1.4050103 -0.5633167 -1.5920533 0.99537516 -0.68801206 -0.12688299 -0.5944905 -0.7191859 1.8682426 -1.093272 -0.313926 -0.62566435]","variable_name":"X_train"}"#;
        
        // Parse the JSON
        let parsed = serde_json::from_str::<serde_json::Value>(test_result).unwrap();
        let value_field = parsed.get("value").unwrap().as_str().unwrap();
        
        // Clean the value
        let cleaned_value = clean_array_string(value_field);
        
        // The cleaned value should not start with "Float32"
        assert!(!cleaned_value.starts_with("Float32"));
        assert!(cleaned_value.starts_with("["));
        
        // Reconstruct the JSON with cleaned value
        let mut cleaned_parsed = parsed.clone();
        cleaned_parsed["value"] = serde_json::Value::String(cleaned_value);
        let cleaned_json = serde_json::to_string(&cleaned_parsed).unwrap();
        
        // The cleaned JSON should not contain "Float32"
        assert!(!cleaned_json.contains("\"Float32["));
        assert!(cleaned_json.contains("\"["));
    }
}

