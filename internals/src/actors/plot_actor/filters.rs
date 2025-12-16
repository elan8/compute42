// Plot data filtering logic
// This module handles filtering out non-plot data

use log::debug;

/// Check if plot data should be filtered out (not actual plot content)
pub fn should_filter_plot_data(data: &str, mime_type: &str) -> bool {
    // Filter out empty data
    if data.trim().is_empty() {
        debug!("[PlotActor] Filtering out empty plot data");
        return true;
    }

    // Only allow through actual plot/image content
    match mime_type {
        "image/svg+xml" => filter_svg_plot(data),
        "image/png" | "image/jpeg" | "image/jpg" | "image/gif" | "image/webp" => filter_image_plot(data),
        "text/html" => filter_html_plot(data),
        "text/plain" => filter_text_plot(data),
        _ => true, // For any other MIME types, be conservative and filter out
    }
}

/// Filter SVG plot data
fn filter_svg_plot(data: &str) -> bool {
    // Filter out very small SVG content that's likely empty
    if data.len() < 100 {
        return true;
    }
    // Filter out if it doesn't contain actual plot elements
    !(data.contains("<rect") || data.contains("<circle") || data.contains("<path") || 
      data.contains("<line") || data.contains("<polygon") || data.contains("<text"))
}

/// Filter binary image plot data
fn filter_image_plot(data: &str) -> bool {
    // For binary image data, we can't easily check content, so filter out
    // if it's too small (likely empty)
    data.len() <= 50
}

/// Filter HTML plot data
fn filter_html_plot(data: &str) -> bool {
    // Filter out if it doesn't contain actual plot content
    !(data.contains("<svg") || data.contains("plotly") || data.contains("chart") || 
      data.contains("canvas") || data.contains("d3"))
}

/// Filter text/plain plot data
fn filter_text_plot(data: &str) -> bool {
    // Filter out JSON data (workspace variables, etc.)
    if data.trim().starts_with('{') && data.trim().ends_with('}') {
        debug!("[PlotActor] Filtering out JSON data (workspace variables): {}", data.chars().take(100).collect::<String>());
        return true;
    }
    
    // Filter out simple values
    if is_simple_value(data) {
        debug!("[PlotActor] Filtering out simple value: {}", data);
        return true;
    }
    
    // Check if it's actual plot data
    let is_plot_data = is_valid_plot_content(data);
    
    if !is_plot_data {
        debug!("[PlotActor] Filtering out text/plain data (not plot data): {}", data.chars().take(100).collect::<String>());
    }
    
    !is_plot_data
}

/// Check if the text contains valid plot content indicators
fn is_valid_plot_content(data: &str) -> bool {
    data.contains("┌") || data.contains("┐") || data.contains("└") || data.contains("┘") || // Box drawing
    data.contains("│") || data.contains("─") || data.contains("┼") || // More box drawing
    data.contains("Plot") || data.contains("Chart") || data.contains("Graph") || // Plot titles
    (data.contains("x") && data.contains("y") && data.len() > 50) || // Data with x,y coordinates
    (data.lines().count() > 5 && data.len() > 100) // Multi-line data
}

/// Check if the data is a simple value (not plot data)
fn is_simple_value(data: &str) -> bool {
    data.len() <= 10 && (
        data == "nothing" || 
        data == "true" || 
        data == "false" || 
        data.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.')
    )
}

