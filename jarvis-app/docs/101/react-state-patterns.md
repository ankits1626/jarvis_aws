# React State Patterns for JarvisApp

This guide explains the state management patterns used in JarvisApp's React frontend, focusing on `useReducer` for complex state and custom hooks for Tauri integration.

## Why useReducer?

JarvisApp uses `useReducer` instead of `useState` because:

1. **Complex state** - Multiple related fields (recordingState, currentRecording, error, etc.)
2. **State transitions** - Clear rules for moving between states (idle → processing → recording)
3. **Atomic updates** - All related fields update together, preventing inconsistent UI
4. **Predictable** - All state changes go through the reducer, making debugging easier

## State Structure

```typescript
export type RecordingState = "idle" | "recording" | "processing";

export interface AppState {
  recordingState: RecordingState;
  currentRecording: string | null;
  recordings: RecordingMetadata[];
  selectedRecording: string | null;
  error: string | null;
  elapsedTime: number;
  showPermissionDialog: boolean;
}
```

**State transition rules:**
- `idle` → `processing`: User clicks record button
- `processing` → `recording`: Backend confirms recording started
- `recording` → `processing`: User clicks stop button
- `processing` → `idle`: Backend confirms recording stopped
- Any state → `idle`: Error occurs

## Actions and Reducer

Actions describe what happened, the reducer decides how to update state:

```typescript
export type AppAction =
  | { type: "START_RECORDING" }
  | { type: "RECORDING_STARTED"; filename: string }
  | { type: "STOP_RECORDING" }
  | { type: "RECORDING_STOPPED" }
  | { type: "SET_RECORDINGS"; recordings: RecordingMetadata[] }
  | { type: "SELECT_RECORDING"; filename: string }
  | { type: "DESELECT_RECORDING" }
  | { type: "REMOVE_RECORDING"; filename: string }
  | { type: "SET_ERROR"; error: string }
  | { type: "CLEAR_ERROR" }
  | { type: "SHOW_PERMISSION_DIALOG" }
  | { type: "HIDE_PERMISSION_DIALOG" }
  | { type: "TICK_TIMER" }
  | { type: "RESET_TIMER" };
```

**Reducer implementation:**
```typescript
export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "START_RECORDING":
      // Transition to processing while waiting for backend
      return {
        ...state,
        recordingState: "processing",
        error: null, // Clear previous errors
      };

    case "RECORDING_STARTED":
      // Backend confirmed - transition to recording
      return {
        ...state,
        recordingState: "recording",
        currentRecording: action.filename,
        error: null,
        elapsedTime: 0, // Reset timer
      };

    case "SET_ERROR":
      // Error occurred - return to idle and show error
      return {
        ...state,
        recordingState: "idle",
        currentRecording: null,
        error: action.error,
        elapsedTime: 0,
      };

    // ... other cases
  }
}
```

## Atomic State Updates

**Why it matters:** Updating multiple fields separately can cause UI inconsistencies. The reducer ensures all related fields update together.

**Example - Starting a recording:**
```typescript
// ❌ BAD: Multiple setState calls (not atomic)
setRecordingState("processing");
setError(null);
// UI might render with old error and new state!

// ✅ GOOD: Single reducer dispatch (atomic)
dispatch({ type: "START_RECORDING" });
// Reducer updates recordingState AND error together
```

## Custom Hooks for Tauri Commands

Custom hooks encapsulate Tauri command invocations and provide a clean API:

### useTauriCommand Hook

Generic hook for invoking Tauri commands with loading and error states:

```typescript
import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

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
        const result = await invoke<T>(commandName, ...args);
        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(errorMessage);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [commandName]
  );

  return [invokeCommand, { loading, error }];
}
```

**Usage:**
```typescript
function MyComponent() {
  const [startRecording, { loading, error }] = useTauriCommand<string, []>('start_recording');
  
  const handleStart = async () => {
    try {
      const filename = await startRecording();
      console.log('Recording started:', filename);
    } catch (err) {
      // Error already captured in hook
    }
  };
  
  return (
    <button onClick={handleStart} disabled={loading}>
      {loading ? 'Starting...' : 'Start Recording'}
    </button>
  );
}
```

### useTauriEvent Hook

Hook for listening to Tauri events from the backend:

```typescript
import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
): void {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    // Set up event listener
    listen<T>(eventName, (event) => {
      handler(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    
    // Clean up on unmount
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [eventName, handler]);
}
```

**Usage:**
```typescript
function MyComponent() {
  const [state, dispatch] = useReducer(appReducer, initialState);
  
  // Listen for recording-started event
  useTauriEvent<{ filename: string }>("recording-started", 
    useCallback((payload) => {
      dispatch({ type: "RECORDING_STARTED", filename: payload.filename });
    }, [])
  );
  
  return <div>Recording: {state.currentRecording}</div>;
}
```

## Complete useRecording Hook

The `useRecording` hook combines everything into a single interface:

```typescript
export function useRecording() {
  const [state, dispatch] = useReducer(appReducer, initialState);

  // Timer for elapsed time (updates every second during recording)
  useEffect(() => {
    if (state.recordingState === "recording") {
      const interval = setInterval(() => {
        dispatch({ type: "TICK_TIMER" });
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [state.recordingState]);

  // Command: Start recording
  const startRecording = useCallback(async () => {
    dispatch({ type: "START_RECORDING" });
    
    try {
      await invoke<string>("start_recording");
      // Backend will emit "recording-started" event
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  // Command: Stop recording
  const stopRecording = useCallback(async () => {
    dispatch({ type: "STOP_RECORDING" });
    
    try {
      await invoke("stop_recording");
      // Backend will emit "recording-stopped" event
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  // Event: Recording started
  useTauriEvent<{ filename: string }>("recording-started", 
    useCallback((payload) => {
      dispatch({ type: "RECORDING_STARTED", filename: payload.filename });
    }, [])
  );

  // Event: Recording stopped
  useTauriEvent("recording-stopped", 
    useCallback(() => {
      dispatch({ type: "RECORDING_STOPPED" });
      refreshRecordings(); // Refresh list to show new recording
    }, [])
  );

  // Event: Permission error
  useTauriEvent<{ message: string }>("permission-error", 
    useCallback((payload) => {
      dispatch({ type: "SET_ERROR", error: payload.message });
      dispatch({ type: "SHOW_PERMISSION_DIALOG" });
    }, [])
  );

  return {
    state,
    startRecording,
    stopRecording,
    // ... other methods
  };
}
```

## Using the Hook in Components

```typescript
function App() {
  const {
    state,
    startRecording,
    stopRecording,
    refreshRecordings,
  } = useRecording();

  // Load recordings on mount
  useEffect(() => {
    refreshRecordings();
  }, [refreshRecordings]);

  return (
    <div>
      <RecordButton
        state={state.recordingState}
        onStart={startRecording}
        onStop={stopRecording}
      />
      
      <StatusIndicator
        state={state.recordingState}
        elapsedTime={state.elapsedTime}
      />
      
      {state.error && (
        <ErrorToast message={state.error} />
      )}
    </div>
  );
}
```

## Key Patterns

**1. Dispatch before invoke:**
```typescript
// Update UI immediately (optimistic update)
dispatch({ type: "START_RECORDING" });

// Then call backend
await invoke("start_recording");
```

**2. Event-driven state updates:**
```typescript
// Backend emits event when operation completes
useTauriEvent("recording-started", (payload) => {
  dispatch({ type: "RECORDING_STARTED", filename: payload.filename });
});
```

**3. Error handling:**
```typescript
try {
  await invoke("start_recording");
} catch (err) {
  // Dispatch error action to update state
  dispatch({ type: "SET_ERROR", error: String(err) });
}
```

**4. useCallback for event handlers:**
```typescript
// Prevent unnecessary re-renders
useTauriEvent("recording-started", 
  useCallback((payload) => {
    dispatch({ type: "RECORDING_STARTED", filename: payload.filename });
  }, []) // Empty deps - dispatch is stable
);
```

## Learn More

- [React useReducer Hook](https://react.dev/reference/react/useReducer) - Official React docs
- [Tauri Events](https://tauri.app/v2/guides/features/events/) - Event system guide
- [React Custom Hooks](https://react.dev/learn/reusing-logic-with-custom-hooks) - Building custom hooks
