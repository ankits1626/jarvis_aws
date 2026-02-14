/**
 * State reducer for JarvisApp
 * 
 * Implements the reducer pattern for managing recording state transitions.
 * All state updates are atomic - all fields are updated together to prevent
 * inconsistent UI states.
 * 
 * State Transition Rules:
 * - idle → processing: User clicks record button
 * - processing → recording: Backend emits recording-started event
 * - recording → processing: User clicks stop button
 * - processing → idle: Backend emits recording-stopped event
 * - Any state → idle: Error occurs
 * 
 * Requirements: 9.1, 9.3, 9.4, 9.5
 */

import type { AppState, AppAction } from './types';

/**
 * Initial application state
 */
export const initialState: AppState = {
  recordingState: "idle",
  currentRecording: null,
  recordings: [],
  selectedRecording: null,
  error: null,
  elapsedTime: 0,
  showPermissionDialog: false,
};

/**
 * Application state reducer
 * 
 * Handles all state transitions based on dispatched actions.
 * Ensures atomic updates - all related fields are updated together.
 * 
 * @param state - Current application state
 * @param action - Action to process
 * @returns New application state
 */
export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "START_RECORDING":
      // Transition to processing state while waiting for backend
      return {
        ...state,
        recordingState: "processing",
        error: null, // Clear any previous errors
      };

    case "RECORDING_STARTED":
      // Backend confirmed recording started - transition to recording state
      return {
        ...state,
        recordingState: "recording",
        currentRecording: action.filename,
        error: null,
        elapsedTime: 0, // Reset timer for new recording
      };

    case "STOP_RECORDING":
      // Transition to processing state while waiting for backend to stop
      return {
        ...state,
        recordingState: "processing",
      };

    case "RECORDING_STOPPED":
      // Backend confirmed recording stopped - return to idle state
      return {
        ...state,
        recordingState: "idle",
        currentRecording: null,
        elapsedTime: 0, // Reset timer
      };

    case "SET_RECORDINGS":
      // Update the list of available recordings
      return {
        ...state,
        recordings: action.recordings,
      };

    case "SELECT_RECORDING":
      // User selected a recording for playback
      return {
        ...state,
        selectedRecording: action.filename,
      };

    case "DESELECT_RECORDING":
      // User closed the audio player
      return {
        ...state,
        selectedRecording: null,
      };

    case "REMOVE_RECORDING":
      // Remove a recording from the list after deletion
      return {
        ...state,
        recordings: state.recordings.filter(
          (r) => r.filename !== action.filename
        ),
        // If the deleted recording was selected, deselect it
        selectedRecording:
          state.selectedRecording === action.filename
            ? null
            : state.selectedRecording,
      };

    case "SET_ERROR":
      // Error occurred - transition to idle state and display error
      return {
        ...state,
        recordingState: "idle",
        currentRecording: null,
        error: action.error,
        elapsedTime: 0, // Reset timer on error
      };

    case "CLEAR_ERROR":
      // User dismissed the error - clear error state
      return {
        ...state,
        error: null,
        showPermissionDialog: false, // Also hide permission dialog
      };

    case "SHOW_PERMISSION_DIALOG":
      // Show permission error dialog
      return {
        ...state,
        showPermissionDialog: true,
      };

    case "HIDE_PERMISSION_DIALOG":
      // Hide permission error dialog
      return {
        ...state,
        showPermissionDialog: false,
      };

    case "TICK_TIMER":
      // Increment elapsed time (called every second during recording)
      return {
        ...state,
        elapsedTime: state.elapsedTime + 1,
      };

    case "RESET_TIMER":
      // Reset elapsed time counter
      return {
        ...state,
        elapsedTime: 0,
      };

    default:
      // TypeScript exhaustiveness check - ensures all actions are handled
      return state;
  }
}
