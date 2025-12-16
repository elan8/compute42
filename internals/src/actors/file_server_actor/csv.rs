use log::debug;
use serde_json::Value;
use csv::ReaderBuilder;

pub fn parse_csv_content(csv_text: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_text.as_bytes());

    let mut rows = Vec::new();
    let mut headers = Vec::new();

    // Read headers
    if let Ok(record) = reader.headers() {
        headers = record.iter().map(|s| s.to_string()).collect();
    }

    // Read data rows
    for result in reader.records() {
        match result {
            Ok(record) => {
                let mut row = Vec::new();
                for field in record.iter() {
                    // Try to parse as number if possible, otherwise keep as string
                    if let Ok(num) = field.parse::<f64>() {
                        row.push(Value::Number(serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))));
                    } else if let Ok(num) = field.parse::<i64>() {
                        row.push(Value::Number(serde_json::Number::from(num)));
                    } else {
                        row.push(Value::String(field.to_string()));
                    }
                }
                rows.push(row);
            }
            Err(e) => {
                debug!("Skipping invalid CSV row: {}", e);
                // Continue parsing other rows
            }
        }
    }

    // Calculate optimal column widths
    let column_widths = calculate_column_widths(&headers, &rows);

    Ok(serde_json::json!({
        "headers": headers,
        "rows": rows,
        "total_rows": rows.len(),
        "column_widths": column_widths
    }))
}

fn calculate_column_widths(headers: &[String], rows: &[Vec<Value>]) -> Vec<u32> {
    let num_columns = headers.len();
    if num_columns == 0 {
        return Vec::new();
    }

    let mut max_widths = vec![0u32; num_columns];
    
    // Calculate width for headers
    for (col_idx, header) in headers.iter().enumerate() {
        let header_width = calculate_text_width(header);
        max_widths[col_idx] = header_width;
    }
    
    // Calculate width for data rows (sample first 1000 rows for performance)
    let sample_size = std::cmp::min(1000, rows.len());
    for row in rows.iter().take(sample_size) {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx < num_columns {
                let cell_width = match cell {
                    Value::String(s) => calculate_text_width(s),
                    Value::Number(n) => calculate_text_width(&n.to_string()),
                    _ => 60, // Default width for other types
                };
                max_widths[col_idx] = std::cmp::max(max_widths[col_idx], cell_width);
            }
        }
    }
    
    // Apply constraints: minimum 80px, maximum 300px
    max_widths.iter().map(|&width| {
        width.clamp(80, 300)
    }).collect()
}

fn calculate_text_width(text: &str) -> u32 {
    // Approximate character width calculation
    // This is a simple approximation - in a real implementation you might want to use
    // a more sophisticated font metrics library
    let mut width = 0u32;
    for ch in text.chars() {
        match ch {
            // Wide characters (CJK, emoji, etc.)
            ch if ch as u32 > 127 => width += 12,
            // Numbers and some symbols
            '0'..='9' | '.' | ',' | '-' | '+' | '$' | '%' => width += 8,
            // Regular letters and spaces
            _ => width += 7,
        }
    }
    
    // Add some padding
    width + 20
}


