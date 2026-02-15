import { useReducer, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { appReducer, initialState } from '../state/reducer';
import { useTauriEvent } from './useTauriEvent';
import type {
  RecordingMetadata,
  RecordingStartedEvent,
  ErrorEvent,
  ShortcutEvent,
  CrashedEvent,
  TranscriptionSegment,
  TranscriptionStoppedEvent,
  TranscriptionErrorEvent,
} from '../state/types';

/**
 * Custom hook for managing recording state and operations.
 * 
 * This hook integrates:
 * - State management via useReducer
 * - Tauri command invocations
 * - Tauri event listeners
 * - Timer management for elapsed time
 * 
 * It provides a complete interface for recording operations including:
 * - Starting and stopping recordings
 * - Listing and refreshing recordings
 * - Selecting and deleting recordings
 * - Error handling and recovery
 * 
 * Requirements: 2.1, 2.4, 2.5, 2.7, 2.8, 6.2, 8.1, 8.2, 8.4
 */
export function useRecording() {
  const [state, dispatch] = useReducer(appReducer, initialState);

  // ===== Timer Management =====

  /**
   * Timer for elapsed time during recording (Requirement 2.7)
   * Increments every second while recording is active
   */
  useEffect(() => {
    if (state.recordingState === "recording") {
      const interval = setInterval(() => {
        dispatch({ type: "TICK_TIMER" });
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [state.recordingState]);

  // ===== Command Invocations =====

  /**
   * Start a new recording
   * Dispatches START_RECORDING action and invokes the backend command
   * Also clears the previous transcript (Requirement 9.6)
   */
  const startRecording = useCallback(async () => {
    dispatch({ type: "START_RECORDING" });
    dispatch({ type: "CLEAR_TRANSCRIPT" }); // Clear transcript on new recording
    
    try {
      await invoke<string>("start_recording");
      // Backend will emit "recording-started" event on success
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  /**
   * Stop the current recording
   * Dispatches STOP_RECORDING action and invokes the backend command
   * Always refreshes recordings list regardless of success or failure
   */
  const stopRecording = useCallback(async () => {
    dispatch({ type: "STOP_RECORDING" });
    
    try {
      await invoke("stop_recording");
      // Backend will emit "recording-stopped" event on success
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
    
    // Always refresh recordings, whether stop succeeded or failed
    // This ensures the UI updates even if stop_recording() returns an error
    // (e.g., when file is 0B or missing)
    try {
      const recordings = await invoke<RecordingMetadata[]>("list_recordings");
      dispatch({ type: "SET_RECORDINGS", recordings });
    } catch (err) {
      // Ignore refresh errors - the main error is already handled above
      console.error("Failed to refresh recordings:", err);
    }
  }, []);

  /**
   * Refresh the list of recordings from the backend
   */
  const refreshRecordings = useCallback(async () => {
    try {
      const recordings = await invoke<RecordingMetadata[]>("list_recordings");
      dispatch({ type: "SET_RECORDINGS", recordings });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  /**
   * Select a recording for playback
   */
  const selectRecording = useCallback((filename: string) => {
    dispatch({ type: "SELECT_RECORDING", filename });
  }, []);

  /**
   * Deselect the current recording
   */
  const deselectRecording = useCallback(() => {
    dispatch({ type: "DESELECT_RECORDING" });
  }, []);

  /**
   * Delete a recording
   * Removes the recording from the backend and updates the list
   */
  const deleteRecording = useCallback(async (filename: string) => {
    try {
      await invoke("delete_recording", { filename });
      dispatch({ type: "REMOVE_RECORDING", filename });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  /**
   * Open system settings for permission configuration
   */
  const openSystemSettings = useCallback(async () => {
    try {
      await invoke("open_system_settings");
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }
  }, []);

  /**
   * Retry recording after fixing permission issues
   * Hides the permission dialog and attempts to start recording again
   */
  const retryRecording = useCallback(async () => {
    dispatch({ type: "HIDE_PERMISSION_DIALOG" });
    await startRecording();
  }, [startRecording]);

  /**
   * Clear the current error
   */
  const clearError = useCallback(() => {
    dispatch({ type: "CLEAR_ERROR" });
  }, []);

  // ===== Event Listeners =====

  /**
   * Listen for recording-started event from backend
   * Emitted when the sidecar process successfully starts
   */
  useTauriEvent<RecordingStartedEvent>("recording-started", 
    useCallback((payload) => {
      dispatch({ type: "RECORDING_STARTED", filename: payload.filename });
    }, [])
  );

  /**
   * Listen for recording-stopped event from backend
   * Emitted when the sidecar process terminates
   */
  useTauriEvent("recording-stopped", 
    useCallback(() => {
      dispatch({ type: "RECORDING_STOPPED" });
      // Refresh recordings list to show the new recording
      refreshRecordings();
    }, [refreshRecordings])
  );

  /**
   * Listen for permission-error event from backend
   * Emitted when the sidecar fails due to missing permissions
   */
  useTauriEvent<ErrorEvent>("permission-error", 
    useCallback((payload) => {
      dispatch({ type: "SET_ERROR", error: payload.message });
      dispatch({ type: "SHOW_PERMISSION_DIALOG" });
    }, [])
  );

  /**
   * Listen for sidecar-error event from backend
   * Emitted when the sidecar writes to stderr (non-permission errors)
   */
  useTauriEvent<ErrorEvent>("sidecar-error", 
    useCallback((payload) => {
      dispatch({ type: "SET_ERROR", error: payload.message });
    }, [])
  );

  /**
   * Listen for sidecar-crashed event from backend
   * Emitted when the sidecar process exits with a non-zero code
   */
  useTauriEvent<CrashedEvent>("sidecar-crashed", 
    useCallback((payload) => {
      const errorMessage = `Recording process crashed${
        payload.code !== null ? ` with code ${payload.code}` : ""
      }`;
      dispatch({ type: "SET_ERROR", error: errorMessage });
    }, [])
  );

  /**
   * Listen for shortcut-triggered event from backend
   * Emitted when the user presses the global shortcut (Cmd+Shift+R)
   */
  useTauriEvent<ShortcutEvent>("shortcut-triggered", 
    useCallback((payload) => {
      if (payload.action === "start") {
        startRecording();
      } else if (payload.action === "stop") {
        stopRecording();
      }
    }, [startRecording, stopRecording])
  );

  /**
   * Listen for transcription-started event from backend
   * Emitted when transcription begins for a recording
   */
  useTauriEvent("transcription-started", 
    useCallback(() => {
      dispatch({ type: "TRANSCRIPTION_STARTED" });
    }, [])
  );

  /**
   * Listen for transcription-update event from backend
   * Emitted when a new transcription segment is produced (partial or final)
   * Backend emits the segment directly, not wrapped in an object
   */
  useTauriEvent<TranscriptionSegment>("transcription-update", 
    useCallback((payload) => {
      dispatch({ type: "TRANSCRIPTION_UPDATE", segment: payload });
    }, [])
  );

  /**
   * Listen for transcription-stopped event from backend
   * Emitted when transcription completes with the full transcript
   */
  useTauriEvent<TranscriptionStoppedEvent>("transcription-stopped", 
    useCallback((payload) => {
      dispatch({ type: "TRANSCRIPTION_STOPPED", transcript: payload.transcript });
    }, [])
  );

  /**
   * Listen for transcription-error event from backend
   * Emitted when a transcription error occurs
   */
  useTauriEvent<TranscriptionErrorEvent>("transcription-error", 
    useCallback((payload) => {
      dispatch({ type: "TRANSCRIPTION_ERROR", message: payload.message });
    }, [])
  );

  // ===== Return Interface =====

  return {
    state,
    startRecording,
    stopRecording,
    selectRecording,
    deselectRecording,
    deleteRecording,
    refreshRecordings,
    openSystemSettings,
    retryRecording,
    clearError,
  };
}
