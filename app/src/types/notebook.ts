// TypeScript types for Jupyter Notebook format (nbformat 4.x)
// These types match the Rust structs in shared/src/frontend/notebook.rs

export type CellType = 'code' | 'markdown' | 'raw';

export interface Notebook {
  nbformat: number;
  nbformat_minor: number;
  metadata: NotebookMetadata;
  cells: NotebookCell[];
}

export interface NotebookMetadata {
  kernelspec?: Kernelspec;
  language_info?: LanguageInfo;
  [key: string]: any; // Allow extra metadata fields
}

export interface Kernelspec {
  display_name: string;
  language: string;
  name: string;
}

export interface LanguageInfo {
  name: string;
  version: string;
}

export interface NotebookCell {
  cell_type: CellType;
  source: string;
  metadata: Record<string, any>;
  outputs: CellOutput[];
  execution_count?: number | null;
}

export type CellOutput = ExecuteResultOutput | DisplayDataOutput | StreamOutput | ErrorOutput;

export interface ExecuteResultOutput {
  output_type: 'execute_result';
  execution_count?: number | null;
  data: OutputData;
  metadata: Record<string, any>;
}

export interface DisplayDataOutput {
  output_type: 'display_data';
  data: OutputData;
  metadata: Record<string, any>;
}

export interface StreamOutput {
  output_type: 'stream';
  name: 'stdout' | 'stderr';
  text: string;
}

export interface ErrorOutput {
  output_type: 'error';
  ename: string;
  evalue: string;
  traceback: string[];
}

export interface OutputData {
  'text/plain'?: string;
  'text/html'?: string;
  'image/png'?: string;
  'image/jpeg'?: string;
  'image/jpg'?: string;
  'application/json'?: any;
  [mimeType: string]: any;
}

// Cell execution state
export type CellExecutionState = 'idle' | 'running' | 'queued' | 'error' | 'success';

// Helper function to check if output is a specific type
export function isExecuteResult(output: CellOutput): output is ExecuteResultOutput {
  return output.output_type === 'execute_result';
}

export function isDisplayData(output: CellOutput): output is DisplayDataOutput {
  return output.output_type === 'display_data';
}

export function isStream(output: CellOutput): output is StreamOutput {
  return output.output_type === 'stream';
}

export function isError(output: CellOutput): output is ErrorOutput {
  return output.output_type === 'error';
}
