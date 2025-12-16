import type { CellOutput, OutputData } from '../types/notebook';
import { logError, debug } from './logger';
import { usePlotStore } from '../store/plotStore';

/**
 * Converts HTTP URLs in notebook outputs to embedded data (base64 for PNG/JPEG, raw XML for SVG).
 * This ensures notebooks are self-contained and portable, following Jupyter notebook format standards.
 *
 * @param outputs - Array of cell outputs to process
 * @returns Promise resolving to outputs with embedded image data
 */
export async function embedImageOutputs(outputs: CellOutput[]): Promise<CellOutput[]> {
  const processedOutputs: CellOutput[] = [];
  let embeddedCount = 0;

  for (const output of outputs) {
    // Only process outputs that have data (execute_result or display_data)
    if (output.output_type === 'execute_result' || output.output_type === 'display_data') {
      const processedData: OutputData = { ...output.data };
      let hasChanges = false;

      // Process each MIME type in the output data
      for (const [mimeType, data] of Object.entries(output.data)) {
        if (typeof data !== 'string') {
          continue; // Skip non-string data
        }

        // Trim whitespace for URL detection
        const trimmedData = data.trim();

        // Check if this is an image MIME type
        if (mimeType === 'image/svg+xml') {
          const embedded = await embedSvgImage(trimmedData);
          if (embedded !== trimmedData) {
            processedData[mimeType] = embedded;
            hasChanges = true;
            embeddedCount++;
          }
        } else if (
          mimeType === 'image/png' ||
          mimeType === 'image/jpeg' ||
          mimeType === 'image/jpg'
        ) {
          const embedded = await embedBinaryImage(trimmedData, mimeType);
          if (embedded !== trimmedData) {
            processedData[mimeType] = embedded;
            hasChanges = true;
            embeddedCount++;
          }
        }
      }

      // Create new output with processed data if changes were made
      if (hasChanges) {
        processedOutputs.push({
          ...output,
          data: processedData,
        });
      } else {
        processedOutputs.push(output);
      }
    } else {
      // Stream and error outputs don't need processing
      processedOutputs.push(output);
    }
  }

  if (embeddedCount > 0) {
    await debug(
      `[NotebookImageEmbedder] Embedded ${embeddedCount} image(s) from ${outputs.length} output(s)`
    );
  }

  return processedOutputs;
}

/**
 * Embeds an SVG image by fetching it from a URL if needed.
 * SVG images in Jupyter notebooks are stored as raw XML strings (not base64).
 *
 * @param data - Either an HTTP URL, data URL, or raw SVG XML string
 * @returns Promise resolving to raw SVG XML string
 */
async function embedSvgImage(data: string): Promise<string> {
  // If it's already raw SVG XML (starts with <svg), clean and return
  if (data.trim().startsWith('<svg')) {
    return cleanSvgData(data);
  }

  // If it's a data URL, extract the SVG content
  if (data.startsWith('data:image/svg+xml')) {
    // Handle both base64 and URL-encoded SVG in data URLs
    if (data.includes(';base64,')) {
      // Base64 encoded SVG in data URL - decode it
      const base64Data = data.split(',')[1];
      try {
        const svgXml = atob(base64Data);
        return svgXml;
      } catch (e) {
        await logError('Failed to decode base64 SVG from data URL', e);
        return data; // Return original on error
      }
    } else if (data.includes(',')) {
      // URL-encoded SVG in data URL - decode it
      const encodedData = data.split(',')[1];
      try {
        const svgXml = decodeURIComponent(encodedData);
        return svgXml;
      } catch (e) {
        await logError('Failed to decode URL-encoded SVG from data URL', e);
        return data; // Return original on error
      }
    }
  }

  // If it's an HTTP URL, try to get from plot store first, then fall back to fetch
  if (data.startsWith('http://') || data.startsWith('https://')) {
    // Try to extract plot ID from URL (format: http://127.0.0.1:PORT/plots/PLOT_ID/image)
    const plotIdMatch = data.match(/\/plots\/([^\/]+)\/image/);
    if (plotIdMatch) {
      const plotId = plotIdMatch[1];

      // Try to get plot from plot store first
      try {
        const plotStore = usePlotStore();
        const plot = await plotStore.getPlot(plotId);
        if (plot && plot.data) {
          // For SVG, the data should be the raw XML string
          if (plot.mime_type === 'image/svg+xml') {
            let svgData = plot.data;

            // If data is base64, decode it
            if (!svgData.trim().startsWith('<svg')) {
              try {
                const decoded = atob(svgData);
                if (decoded.trim().startsWith('<svg')) {
                  svgData = decoded;
                }
              } catch (e) {
                // Not base64, use as-is
              }
            }

            // Clean up any HTML remnants (e.g., leftover img tags from previous processing)
            svgData = cleanSvgData(svgData);

            return svgData;
          }
        }
      } catch (error) {
        // Silently fall through to HTTP fetch
      }
    }

    // Fall back to HTTP fetch
    try {
      const response = await fetch(data);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const svgXml = await response.text();
      // Clean and validate it's actually SVG
      const cleanedSvg = cleanSvgData(svgXml);
      if (cleanedSvg.trim().startsWith('<svg')) {
        return cleanedSvg;
      } else {
        throw new Error('Fetched content is not valid SVG XML');
      }
    } catch (error) {
      await logError(`Failed to fetch SVG from URL: ${data}`, error);
      // Return original URL on error (better than losing the output)
      return data;
    }
  }

  // If it's already base64 (without data: prefix), try to decode it
  // This handles cases where base64 SVG might be stored directly
  try {
    const decoded = atob(data);
    if (decoded.trim().startsWith('<svg')) {
      return cleanSvgData(decoded);
    }
  } catch (e) {
    // Not valid base64, continue
  }

  // Unknown format, try to clean it anyway
  return cleanSvgData(data);
}

/**
 * Embeds a binary image (PNG/JPEG) by fetching it from a URL if needed.
 * Binary images in Jupyter notebooks are stored as base64 strings (without data: prefix).
 *
 * @param data - Either an HTTP URL, data URL, or base64 string
 * @param mimeType - The MIME type of the image (e.g., 'image/png', 'image/jpeg')
 * @returns Promise resolving to base64 string (without data: prefix)
 */
async function embedBinaryImage(data: string, mimeType: string): Promise<string> {
  // If it's already a base64 string (no data: prefix), return as-is
  // Base64 strings are typically alphanumeric with +, /, = characters
  if (!data.startsWith('data:') && !data.startsWith('http://') && !data.startsWith('https://')) {
    // Check if it looks like base64
    const base64Pattern = /^[A-Za-z0-9+/=]+$/;
    if (base64Pattern.test(data)) {
      return data;
    }
  }

  // If it's a data URL, extract the base64 part
  if (data.startsWith('data:')) {
    const parts = data.split(',');
    if (parts.length === 2) {
      const base64Data = parts[1];
      return base64Data;
    }
    // Malformed data URL, return as-is
    return data;
  }

  // If it's an HTTP URL, try to get from plot store first, then fall back to fetch
  if (data.startsWith('http://') || data.startsWith('https://')) {
    // Try to extract plot ID from URL (format: http://127.0.0.1:PORT/plots/PLOT_ID/image)
    const plotIdMatch = data.match(/\/plots\/([^\/]+)\/image/);
    if (plotIdMatch) {
      const plotId = plotIdMatch[1];

      // Try to get plot from plot store first
      try {
        const plotStore = usePlotStore();
        const plot = await plotStore.getPlot(plotId);
        if (plot && plot.data) {
          // For binary images, the data should be base64
          // Remove data: prefix if present
          const base64Data = plot.data.replace(/^data:.*,/, '');
          return base64Data;
        }
      } catch (error) {
        // Silently fall through to HTTP fetch
      }
    }

    // Fall back to HTTP fetch
    try {
      const response = await fetch(data);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const blob = await response.blob();

      // Convert blob to base64
      const base64 = await blobToBase64(blob);
      return base64;
    } catch (error) {
      await logError(`Failed to fetch image from URL: ${data}`, error);
      // Return original URL on error (better than losing the output)
      return data;
    }
  }

  // Unknown format, return as-is
  return data;
}

/**
 * Cleans SVG data by removing any HTML remnants (e.g., leftover img tags).
 * Extracts only the SVG XML content.
 *
 * @param data - SVG data that might contain HTML remnants
 * @returns Clean SVG XML string
 */
function cleanSvgData(data: string): string {
  // If it's already clean SVG XML, return as-is
  const trimmed = data.trim();
  if (trimmed.startsWith('<svg')) {
    // Check if there are HTML remnants after the SVG closing tag
    const svgEndIndex = trimmed.lastIndexOf('</svg>');
    if (svgEndIndex !== -1) {
      // Extract only up to the closing </svg> tag
      const cleanSvg = trimmed.substring(0, svgEndIndex + 6);
      // Verify it's valid SVG
      if (cleanSvg.trim().startsWith('<svg') && cleanSvg.trim().endsWith('</svg>')) {
        return cleanSvg;
      }
    }
    return trimmed;
  }

  // Try to extract SVG from HTML img tag
  const imgMatch = data.match(/<img[^>]*src=["']([^"']+)["'][^>]*>/);
  if (imgMatch && imgMatch[1]) {
    const src = imgMatch[1];
    // If src is a data URL with SVG, extract it
    if (src.startsWith('data:image/svg+xml')) {
      const svgPart = src.split(',')[1];
      try {
        // Try URL decoding first
        const decoded = decodeURIComponent(svgPart);
        if (decoded.trim().startsWith('<svg')) {
          return decoded;
        }
      } catch (e) {
        // Not URL encoded, try base64
        try {
          const decoded = atob(svgPart);
          if (decoded.trim().startsWith('<svg')) {
            return decoded;
          }
        } catch (e2) {
          // Not base64 either
        }
      }
    }
  }

  // Try to find SVG content in the string
  const svgMatch = data.match(/<svg[\s\S]*?<\/svg>/);
  if (svgMatch) {
    return svgMatch[0];
  }

  // Return as-is if we can't clean it
  return data;
}

/**
 * Converts a Blob to a base64 string.
 *
 * @param blob - The blob to convert
 * @returns Promise resolving to base64 string (without data: prefix)
 */
function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onloadend = () => {
      const result = reader.result as string;
      // Remove data: prefix if present
      const base64 = result.includes(',') ? result.split(',')[1] : result;
      resolve(base64);
    };
    reader.onerror = reject;
    reader.readAsDataURL(blob);
  });
}
