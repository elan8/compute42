// app/src/store/terminalStore.ts
import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { debug, logError } from '../utils/logger';

interface TerminalState {
  activeTerminalId: string | null;
  globalStreamInitialized: boolean;
  juliaOutputBuffer: string;
  isListening: boolean;
  eventUnlistenFn: (() => void) | null;
  errorUnlistenFn: (() => void) | null;
  isBusy: boolean; // Track when code is being executed
  terminalSerializedState: string | null; // Serialized terminal state (in-memory only)
  hasShownInitialPrompt: boolean; // Track if initial prompt was shown (persists across component remounts)
}

interface StreamOutput {
  content: string;
  stream_type: 'Stdout' | 'Stderr';
  timestamp: number;
}

export const useTerminalStore = defineStore('terminal', {
  state: (): TerminalState => ({
    activeTerminalId: null,
    globalStreamInitialized: false,
    juliaOutputBuffer: '',
    isListening: false,
    eventUnlistenFn: null,
    errorUnlistenFn: null,
    isBusy: false,
    terminalSerializedState: null,
    hasShownInitialPrompt: false,
  }),
  actions: {
    setActiveTerminalId(id: string): void {
      this.activeTerminalId = id;
    },
    clearActiveTerminalId(expectedId: string | null): void {
      // Only clear if the expectedId matches, to prevent an old terminal instance
      // from clearing an ID set by a newer one.
      if (this.activeTerminalId === expectedId) {
        this.activeTerminalId = null;
      }
    },

    setBusy(busy: boolean): void {
      this.isBusy = busy;
      //console.log(`terminalStore: isBusy set to ${this.isBusy}`);
    },

    getBusy(): boolean {
      return this.isBusy;
    },

    // Initialize global Julia stream capture (now handled automatically by named pipe manager)
    async initializeGlobalStream(): Promise<void> {
      if (this.globalStreamInitialized) {
        return;
      }

      try {
        this.globalStreamInitialized = true;
        this.startGlobalStreamListening();
      } catch (error) {
        await logError('Failed to initialize global Julia streams:', error);
        throw error;
      }
    },

    // Start listening to global Julia output streams using events
    async startGlobalStreamListening(): Promise<void> {
      if (this.isListening) {
        return;
      }

      this.isListening = true;

      // Set up event listener for Julia output from named pipe manager
      try {
        const unlisten = await listen<StreamOutput[]>('julia-output', (event) => {
          this.handleGlobalOutputEvent(event.payload);
        });

        this.eventUnlistenFn = unlisten;
        //console.log('Started listening for Julia output events');
      } catch (error) {
        await logError('Failed to set up Julia output event listener:', error);
        this.isListening = false;
        throw error;
      }

      // Set up event listener for Julia daemon errors (stderr) - this comes from the daemon
      try {
        const errorUnlisten = await listen<string>('julia-daemon-error', (event) => {
          this.handleDaemonErrorEvent(event.payload);
        });

        this.errorUnlistenFn = errorUnlisten;
        //console.log('Started listening for Julia daemon error events');
      } catch (error) {
        await logError('Failed to set up Julia daemon error event listener:', error);
        // Don't throw here as the main output listener is more important
      }
    },

    // Handle daemon output events from Julia (legacy - keeping for compatibility)
    handleDaemonOutputEvent(output: string): void {
      if (output) {
        this.addToOutputBuffer(output, 'stdout');
      }
    },

    // Handle daemon error events from Julia (stderr)
    handleDaemonErrorEvent(error: string): void {
      if (error) {
        this.addToOutputBuffer(error, 'stderr');
      }
    },

    // Handle global output events from Julia (from named pipe manager)
    handleGlobalOutputEvent(outputs: StreamOutput[]): void {
      for (const output of outputs) {
        if (output.content) {
          const streamType = output.stream_type;
          // Only stdout and stderr should appear in the REPL
          const streamTypeStr = streamType === 'Stdout' ? 'stdout' : 'stderr';
          this.addToOutputBuffer(output.content, streamTypeStr);
        }
      }
    },

    // Add output to the buffer
    addToOutputBuffer(output: string, streamType: 'stdout' | 'stderr'): void {
      // Add timestamp and stream type prefix
      const timestamp = new Date().toLocaleTimeString();
      // Don't prefix stderr with [ERROR] since Julia uses it for many non-error messages
      const formattedOutput = `${timestamp} ${output}`;

      this.juliaOutputBuffer += formattedOutput;

      // Emit event for real-time updates
      this.emitOutputUpdate(formattedOutput, streamType);
    },

    // Emit output update event
    emitOutputUpdate(_output: string, _streamType: 'stdout' | 'stderr'): void {
      // This would emit an event that the terminal component can listen to
      // For now, we'll just update the buffer
      // In a real implementation, you might use a custom event system
      //console.log(`[${streamType.toUpperCase()}] ${output}`);
    },

    // Execute Julia code (simplified - no per-command capture)
    async executeJuliaCode(code: string): Promise<string> {
      try {
        // Ensure global streams are initialized
        if (!this.globalStreamInitialized) {
          await this.initializeGlobalStream();
        }

        // Execute code - output will appear via global stream events
        const result = await invoke<string>('execute_julia_code', { code });

        // Return the result, but output is handled by global stream events
        return result;
      } catch (error) {
        await logError('Failed to execute Julia code:', error);
        throw error;
      }
    },

    // Execute Julia file (simplified - no per-command capture)
    async executeJuliaFile(filePath: string, fileContent: string): Promise<string> {
      try {
        // Ensure global streams are initialized
        if (!this.globalStreamInitialized) {
          await this.initializeGlobalStream();
        }

        // Execute file using the new file execution command - output will appear via global stream events
        const result = await invoke<string>('execute_julia_file', { filePath, fileContent });

        // Return the result, but output is handled by global stream events
        return result;
      } catch (error) {
        await logError('Failed to execute Julia file:', error);
        throw error;
      }
    },

    // Execute Julia file with capture (for terminal display)
    async executeJuliaFileWithCapture(
      filePath: string,
      fileContent: string
    ): Promise<{ result: string; stdout: string; stderr: string }> {
      try {
        // Ensure global streams are initialized
        if (!this.globalStreamInitialized) {
          await this.initializeGlobalStream();
        }

        // Execute file using the new file execution command - output will appear via global stream events
        const result = await invoke<string>('execute_julia_file', { filePath, fileContent });

        // For now, return a simple structure - the actual output will come via global stream events
        return {
          result: result,
          stdout: '', // Output will come via global stream events
          stderr: '', // Output will come via global stream events
        };
      } catch (error) {
        await logError('Failed to execute Julia file with capture:', error);
        throw error;
      }
    },

    // Get current output buffer
    getOutputBuffer(): string {
      return this.juliaOutputBuffer;
    },

    // Clear output buffer
    clearOutputBuffer(): void {
      this.juliaOutputBuffer = '';
    },

    // Stop listening to global streams
    async stopGlobalStreamListening(): Promise<void> {
      this.isListening = false;

      // Unlisten from events
      if (this.eventUnlistenFn) {
        this.eventUnlistenFn();
        this.eventUnlistenFn = null;
        await debug('Stopped listening for Julia output events');
      }

      if (this.errorUnlistenFn) {
        this.errorUnlistenFn();
        this.errorUnlistenFn = null;
        await debug('Stopped listening for Julia daemon error events');
      }
    },

    // Check if global streams are initialized
    isGlobalStreamInitialized(): boolean {
      return this.globalStreamInitialized;
    },

    // Check if currently listening
    isCurrentlyListening(): boolean {
      return this.isListening;
    },

    // Terminal serialized state management (in-memory only, no localStorage)
    setTerminalSerializedState(state: string): void {
      this.terminalSerializedState = state;
    },

    getTerminalSerializedState(): string | null {
      return this.terminalSerializedState;
    },

    clearTerminalSerializedState(): void {
      this.terminalSerializedState = null;
    },

    // Initial prompt management (persists across component remounts)
    setHasShownInitialPrompt(shown: boolean): void {
      this.hasShownInitialPrompt = shown;
    },

    getHasShownInitialPrompt(): boolean {
      return this.hasShownInitialPrompt;
    },
  },
});
