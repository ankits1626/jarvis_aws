import { useEffect } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

/**
 * Custom hook for listening to Tauri events.
 * Sets up an event listener on mount and cleans it up on unmount.
 * 
 * @template T - The type of the event payload
 * @param eventName - The name of the Tauri event to listen for
 * @param handler - The callback function to handle the event payload
 */
export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
): void {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    // Set up the event listener
    const setupListener = async () => {
      unlisten = await listen<T>(eventName, (event) => {
        handler(event.payload);
      });
    };

    setupListener();

    // Clean up the listener on unmount
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [eventName, handler]);
}
