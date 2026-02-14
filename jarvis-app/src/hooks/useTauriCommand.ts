import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * Custom hook for invoking Tauri commands with loading and error state management.
 * 
 * @template T - The return type of the command
 * @template Args - The argument types for the command
 * @param commandName - The name of the Tauri command to invoke
 * @returns A tuple containing the invoke function and state object with loading and error
 */
export function useTauriCommand<T, Args extends any[]>(
  commandName: string
): [
  (...args: Args) => Promise<T>,
  { loading: boolean; error: string | null }
] {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const invokeCommand = useCallback(
    async (...args: Args): Promise<T> => {
      setLoading(true);
      setError(null);

      try {
        const result = await invoke<T>(commandName, 
          args.length > 0 ? args[0] : undefined
        );
        setLoading(false);
        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(errorMessage);
        setLoading(false);
        throw err;
      }
    },
    [commandName]
  );

  return [invokeCommand, { loading, error }];
}
