import { invoke } from '@tauri-apps/api/core';
import { unifiedEventService, EventCategory } from './unifiedEventService';
import type {
  CellOutput,
  ExecuteResultOutput,
  DisplayDataOutput,
  StreamOutput,
  ErrorOutput,
  OutputData,
} from '../types/notebook';
import { debug, logError } from '../utils/logger';

/**
 * Service for executing notebook cells and capturing outputs
 */
export class NotebookExecutionService {
  private executionCount = 0;
  private activeExecutions: Map<
    string,
    {
      resolve: (outputs: CellOutput[]) => void;
      reject: (error: Error) => void;
      outputs: CellOutput[];
      stdout: string[];
      stderr: string[];
      plots: Array<{ id: string; mimeType: string; data: string }>;
      startTime: number;
    }
  > = new Map();
  private currentExecutingCell: string | null = null;

  constructor() {
    this.setupEventListeners();
  }

  /**
   * Setup event listeners for capturing execution outputs
   */
  private async setupEventListeners() {
    // Listen for plot events
    await unifiedEventService.addEventListener(EventCategory.Plot, 'plot-added', async (event) => {
      const payload = event.payload;
      if (payload && payload.plot_data) {
        const plotData = payload.plot_data;
        // Find the most recently started execution (current cell being executed)
        // This is a simple heuristic - in a more sophisticated implementation,
        // we could track which cell is currently executing
        const executions = Array.from(this.activeExecutions.entries());
        if (executions.length > 0) {
          // Get the most recently added execution (last in map iteration order)
          const [cellId, execution] = executions[executions.length - 1];

          // Plot data can be either base64 data or a URL
          const plotId = plotData.id || plotData.plot_id;
          const mimeType = plotData.mime_type || 'image/png';
          // Prefer image_url if available (for HTTP URLs), otherwise use data (base64)
          const data = plotData.image_url || plotData.data || '';

          execution.plots.push({
            id: plotId,
            mimeType: mimeType,
            data: data,
          });
        }
      }
    });

    // Listen for Julia output events (stdout/stderr)
    // Use 'output-detailed' which is always emitted, even when terminal output is suppressed
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'output-detailed',
      async (event) => {
        const outputs = event.payload;
        if (Array.isArray(outputs)) {
          // Only add output to the currently executing cell to prevent concatenation
          // If no cell is currently executing, add to the most recently started execution
          let targetCellId: string | null = this.currentExecutingCell;

          if (!targetCellId && this.activeExecutions.size > 0) {
            // Find the most recently started execution (highest startTime)
            let mostRecent: { cellId: string; startTime: number } | null = null;
            for (const [cellId, execution] of this.activeExecutions.entries()) {
              if (!mostRecent || execution.startTime > mostRecent.startTime) {
                mostRecent = { cellId, startTime: execution.startTime };
              }
            }
            if (mostRecent) {
              targetCellId = mostRecent.cellId;
            }
          }

          if (targetCellId) {
            const execution = this.activeExecutions.get(targetCellId);
            if (execution) {
              for (const output of outputs) {
                const streamType = output.stream_type || output.stream;
                const content = output.content || output.text;

                if (streamType === 'stdout' || streamType === 'Stdout') {
                  execution.stdout.push(content);
                } else if (streamType === 'stderr' || streamType === 'Stderr') {
                  execution.stderr.push(content);
                }
              }
            }
          }
        }
      }
    );
  }

  /**
   * Execute a notebook cell and return Jupyter-formatted outputs
   */
  async executeCell(cellId: string, code: string, notebookPath?: string): Promise<CellOutput[]> {
    this.executionCount++;
    const executionCount = this.executionCount;

    return new Promise<CellOutput[]>((resolve, reject) => {
      const outputs: CellOutput[] = [];
      const stdout: string[] = [];
      const stderr: string[] = [];
      const plots: Array<{ id: string; mimeType: string; data: string }> = [];

      // Clear any previous execution state for this cell
      this.activeExecutions.delete(cellId);

      // Set this as the currently executing cell BEFORE storing execution state
      // This ensures output goes to the right cell from the start
      this.currentExecutingCell = cellId;

      // Store execution state
      const startTime = Date.now();
      this.activeExecutions.set(cellId, {
        resolve,
        reject,
        outputs,
        stdout,
        stderr,
        plots,
        startTime,
      });

      // Execute notebook cell (emits output-detailed events and notebook-specific events)
      invoke<string>('execute_notebook_cell', { cellId, code, notebookPath: notebookPath || null })
        .then(async (result) => {
          // Wait briefly for any plots/outputs to arrive via events
          // Plots are emitted asynchronously and may arrive slightly after execution completes
          await new Promise((resolve) => setTimeout(resolve, 500));
          this.finalizeExecution(cellId, executionCount, result, null);
        })
        .catch((error) => {
          this.finalizeExecution(cellId, executionCount, null, error);
        });
    });
  }

  /**
   * Execute an entire notebook file (backend-driven) and return a map of cellId -> outputs.
   * The backend emits per-cell events carrying outputs; this method aggregates them.
   */
  async executeNotebookFile(
    path: string,
    cellIdsInOrder: (string | undefined)[]
  ): Promise<Map<string, CellOutput[]>> {
    const outputsMap = new Map<string, CellOutput[]>();

    return new Promise<Map<string, CellOutput[]>>(async (resolve, reject) => {
      let completed = false;

      const cleanup = async () => {
        try {
          await unifiedEventService.removeEventListener(
            EventCategory.Julia,
            'notebook-cell-output'
          );
          await unifiedEventService.removeEventListener(EventCategory.Julia, 'notebook-complete');
        } catch (error) {
          await logError('[NotebookExecutionService] Error during cleanup', error);
        }
      };

      const handleCellOutput = async (event: any) => {
        const payload = event.payload;
        if (!payload) return;
        await debug(
          `[NotebookExecutionService] Received notebook-cell-output event: cell_id=${payload.cell_id}, cell_index=${payload.cell_index}, outputs_count=${Array.isArray(payload.outputs) ? payload.outputs.length : 0}`
        );

        // Prioritize cell_index over cell_id - cell_index is the actual index in the notebook
        // which matches the order in cellIdsInOrder
        const cellIndex = payload.cell_index;
        let cellId: string;

        if (
          typeof cellIndex === 'number' &&
          cellIndex < cellIdsInOrder.length &&
          cellIdsInOrder[cellIndex] !== undefined
        ) {
          // Use the actual cell ID from the frontend's cellIdsInOrder array (sparse array indexed by full notebook index)
          cellId = cellIdsInOrder[cellIndex]!;
          await debug(
            `[NotebookExecutionService] Matched by index ${cellIndex} to cell ID: ${cellId}`
          );
        } else {
          // Fallback: try to use cell_id from payload, or generate one
          cellId =
            payload.cell_id ||
            payload.cellId ||
            (typeof cellIndex === 'number' ? `cell-${cellIndex}` : 'unknown');
          await debug(
            `[NotebookExecutionService] Using fallback cell ID: ${cellId} (cellIndex: ${cellIndex}, cellIdsInOrder length: ${cellIdsInOrder.length})`
          );
        }

        const outputs = (payload.outputs as CellOutput[]) || [];
        outputsMap.set(cellId, outputs);
        await debug(
          `[NotebookExecutionService] Updated outputsMap for cell: ${cellId}, outputs: ${outputs.length}`
        );
      };

      const handleComplete = async () => {
        if (completed) return;
        await debug(
          `[NotebookExecutionService] Received notebook-complete event, resolving with ${outputsMap.size} cells`
        );
        completed = true;
        await cleanup();
        resolve(outputsMap);
      };

      try {
        // Register listeners for per-cell output and completion BEFORE invoking backend
        // This ensures listeners are ready before events start arriving
        await debug(
          `[NotebookExecutionService] Setting up event listeners for notebook execution: path=${path}, cellIds=${cellIdsInOrder.join(',')}`
        );

        // Set up cell output handler
        const cellOutputHandler = async (event: any) => {
          await debug(`[NotebookExecutionService] handleCellOutput called`);
          await handleCellOutput(event);
        };

        // Set up completion handler
        const completeHandler = async (event: any) => {
          await debug(`[NotebookExecutionService] handleComplete called`);
          await handleComplete();
        };

        await unifiedEventService.addEventListener(
          EventCategory.Julia,
          'notebook-cell-output',
          cellOutputHandler
        );
        await debug(`[NotebookExecutionService] Added listener for julia:notebook-cell-output`);

        await unifiedEventService.addEventListener(
          EventCategory.Julia,
          'notebook-complete',
          completeHandler
        );
        await debug(`[NotebookExecutionService] Added listener for julia:notebook-complete`);

        await debug(
          `[NotebookExecutionService] Event listeners set up, invoking backend command...`
        );

        // Invoke backend command AFTER listeners are set up
        await invoke<void>('execute_notebook_file', { path });

        await debug(
          `[NotebookExecutionService] Backend command invoked, waiting for events... Current outputsMap size: ${outputsMap.size}`
        );

        // Note: The promise will resolve when handleComplete is called
        // We don't need a timeout here because the backend will always emit notebook-complete
        // But we'll add a safety timeout just in case
        setTimeout(async () => {
          if (!completed) {
            await logError(
              `[NotebookExecutionService] Timeout waiting for notebook-complete event, resolving anyway. Final outputsMap size: ${outputsMap.size}, entries: ${JSON.stringify(Array.from(outputsMap.entries()))}`
            );
            completed = true;
            await cleanup();
            resolve(outputsMap);
          }
        }, 300000); // 5 minutes safety timeout
      } catch (error) {
        await logError('[NotebookExecutionService] Error during notebook execution', error);
        if (!completed) {
          completed = true;
          await cleanup();
          reject(error);
        }
      }
    });
  }

  /**
   * Execute multiple notebook cells in batch (emits busy/done only at start/end)
   * Returns a map of cellId -> outputs
   */
  async executeCellsBatch(
    cells: Array<{ cellId: string; code: string }>,
    notebookPath?: string
  ): Promise<Map<string, CellOutput[]>> {
    if (cells.length === 0) {
      return new Map();
    }

    // Execute cells sequentially from frontend to properly track currentExecutingCell
    // This ensures output goes to the correct cell
    const outputsMap = new Map<string, CellOutput[]>();

    try {
      for (const cell of cells) {
        // Set up execution tracking for this cell
        this.executionCount++;
        const outputs: CellOutput[] = [];
        const stdout: string[] = [];
        const stderr: string[] = [];
        const plots: Array<{ id: string; mimeType: string; data: string }> = [];

        // Clear any previous execution state for this cell
        this.activeExecutions.delete(cell.cellId);

        // Set this as the currently executing cell BEFORE storing execution state
        this.currentExecutingCell = cell.cellId;

        // Store execution state for event handling (outputs will be captured via events)
        const startTime = Date.now();
        this.activeExecutions.set(cell.cellId, {
          resolve: () => {},
          reject: () => {},
          outputs,
          stdout,
          stderr,
          plots,
          startTime,
        });

        try {
          // Execute this cell individually
          const result = await invoke<string>('execute_notebook_cell', {
            cellId: cell.cellId,
            code: cell.code,
            notebookPath: notebookPath || null,
          });

          // Wait briefly for any plots/outputs to arrive via events
          await new Promise((resolve) => setTimeout(resolve, 100));

          // Finalize this cell's execution
          this.finalizeExecution(cell.cellId, this.executionCount, result, null);

          // Get the final outputs
          const execution = this.activeExecutions.get(cell.cellId);
          if (execution) {
            outputsMap.set(cell.cellId, execution.outputs);
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          this.finalizeExecution(cell.cellId, this.executionCount, null, error);

          const execution = this.activeExecutions.get(cell.cellId);
          if (execution) {
            outputsMap.set(cell.cellId, execution.outputs);
          }
        } finally {
          // Clear current executing cell after this cell completes
          if (this.currentExecutingCell === cell.cellId) {
            this.currentExecutingCell = null;
          }
          // Clean up this cell's execution state
          this.activeExecutions.delete(cell.cellId);
        }
      }

      return outputsMap;
    } catch (error) {
      // Clean up on error
      for (const cellId of cells.map((c) => c.cellId)) {
        this.activeExecutions.delete(cellId);
      }
      throw error;
    }
  }

  /**
   * Finalize execution and format outputs
   */
  private finalizeExecution(
    cellId: string,
    executionCount: number,
    result: string | null,
    error: any
  ) {
    const execution = this.activeExecutions.get(cellId);
    if (!execution) {
      debug(`[NotebookExecution] No execution found for cell ${cellId}`);
      return;
    }

    const { outputs, stdout, stderr, plots } = execution;

    // Add stdout stream output
    if (stdout.length > 0) {
      const stdoutText = stdout.join('');
      if (stdoutText.trim()) {
        outputs.push({
          output_type: 'stream',
          name: 'stdout',
          text: stdoutText,
        } as StreamOutput);
      }
    }

    // Add stderr stream output
    if (stderr.length > 0) {
      const stderrText = stderr.join('');
      if (stderrText.trim()) {
        outputs.push({
          output_type: 'stream',
          name: 'stderr',
          text: stderrText,
        } as StreamOutput);
      }
    }

    // Add plot outputs
    for (const plot of plots) {
      const outputData: OutputData = {};
      // Use the actual mime type from the plot, including SVG
      if (
        plot.mimeType &&
        (plot.mimeType.startsWith('image/') || plot.mimeType === 'image/svg+xml')
      ) {
        outputData[plot.mimeType] = plot.data;
      } else {
        // Default to PNG if mime type is not recognized
        outputData['image/png'] = plot.data;
      }

      outputs.push({
        output_type: 'display_data',
        data: outputData,
        metadata: {},
      } as DisplayDataOutput);
    }

    // Handle error
    if (error) {
      const errorMessage = typeof error === 'string' ? error : error.message || 'Execution failed';
      const traceback =
        typeof error === 'string'
          ? [errorMessage]
          : error.stack
            ? error.stack.split('\n')
            : [errorMessage];

      outputs.push({
        output_type: 'error',
        ename: 'Error',
        evalue: errorMessage,
        traceback,
      } as ErrorOutput);

      execution.reject(new Error(errorMessage));
    } else {
      // Add execute result if we have a result
      if (result && result.trim()) {
        const outputData: OutputData = {
          'text/plain': result,
        };

        outputs.push({
          output_type: 'execute_result',
          execution_count: executionCount,
          data: outputData,
          metadata: {},
        } as ExecuteResultOutput);
      }

      execution.resolve(outputs);
    }

    // Clean up
    if (this.currentExecutingCell === cellId) {
      this.currentExecutingCell = null;
    }
    this.activeExecutions.delete(cellId);
  }

  /**
   * Cancel execution for a cell
   */
  cancelExecution(cellId: string) {
    const execution = this.activeExecutions.get(cellId);
    if (execution) {
      execution.reject(new Error('Execution cancelled'));
      this.activeExecutions.delete(cellId);
    }
  }

  /**
   * Get current execution count
   */
  getExecutionCount(): number {
    return this.executionCount;
  }
}

// Singleton instance
export const notebookExecutionService = new NotebookExecutionService();
