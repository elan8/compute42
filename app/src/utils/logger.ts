import { trace, debug, info, warn, error } from '@tauri-apps/plugin-log';

// Re-export the logging functions
export { trace, debug, info, warn, error };

// Helper function to log objects with proper stringification
export async function logObject(
  level: 'trace' | 'debug' | 'info' | 'warn' | 'error',
  message: string,
  obj: any
) {
  const logFn = { trace, debug, info, warn, error }[level];
  await logFn(`${message}: ${JSON.stringify(obj, null, 2)}`);
}

// Helper function to log errors with stack traces
export async function logError(message: string, err: any) {
  const errorMessage = err instanceof Error ? `${err.message}\n${err.stack}` : String(err);
  await error(`${message}: ${errorMessage}`);
}
