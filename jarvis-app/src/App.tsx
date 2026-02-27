import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onAction } from "@tauri-apps/plugin-notification";
import { useRecording } from "./hooks/useRecording";
import { useResizable } from "./hooks/useResizable";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { DeleteConfirmDialog } from "./components/DeleteConfirmDialog";
import { ErrorToast } from "./components/ErrorToast";
import { Settings } from "./components/Settings";
import { YouTubeSection } from "./components/YouTubeSection";
import { BrowserTool } from "./components/BrowserTool";
import { GemsPanel } from "./components/GemsPanel";
import LeftNav from "./components/LeftNav";
import RightPanel from "./components/RightPanel";
import type { YouTubeDetectedEvent, TranscriptResult, RecordingTranscriptionState, GemPreview, AvailabilityResult, Gem } from "./state/types";
import "./App.css";

type ActiveNav = 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings';

/**
 * Main application component for JarvisApp
 * 
 * Integrates all features:
 * - Recording management (start/stop)
 * - Recordings list display
 * - Audio playback
 * - Error handling and permission dialogs
 * - Global shortcut support
 * 
 * Requirements: 1.2, 2.1, 2.4, 2.7, 2.8, 4.1, 4.2, 4.3, 5.3, 6.1, 8.4
 */
function App() {
  const {
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
  } = useRecording();

  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [isLoadingRecordings, setIsLoadingRecordings] = useState(true);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [youtubeNotification, setYoutubeNotification] = useState(false);
  const [toastError, setToastError] = useState<string | null>(null);
  
  // Three-panel layout state
  const [activeNav, setActiveNav] = useState<ActiveNav>('record');
  const [leftNavCollapsed, setLeftNavCollapsed] = useState(false);
  const [selectedGemId, setSelectedGemId] = useState<string | null>(null);
  const [gemsPanelRefreshKey, setGemsPanelRefreshKey] = useState(0);
  
  // Resizable right panel
  const { width: rightPanelWidth, handleMouseDown: handleResizeMouseDown, isResizing } = useResizable();
  const showRightPanel = activeNav === 'record' || activeNav === 'recordings' || activeNav === 'gems';

  // Recording transcription state
  const [recordingStates, setRecordingStates] = useState<Record<string, RecordingTranscriptionState>>({});
  const [aiAvailable, setAiAvailable] = useState<boolean>(false);

  // Load recordings on mount (Requirement 1.2)
  useEffect(() => {
    const loadRecordings = async () => {
      setIsLoadingRecordings(true);
      await refreshRecordings();
      setIsLoadingRecordings(false);
    };
    loadRecordings();
  }, [refreshRecordings]);
  
  // Check AI availability and gem status for all recordings on mount
  // Uses recordings.length to avoid re-running when recording objects change (e.g., selection)
  // This prevents resetting transcription/gem states during user interactions
  useEffect(() => {
    const checkAvailabilityAndGems = async () => {
      try {
        // Check AI availability
        const availability = await invoke<AvailabilityResult>('check_intel_availability');
        setAiAvailable(availability.available);

        // Batch check gem status for all recordings
        if (state.recordings.length > 0) {
          const filenames = state.recordings.map(r => r.filename);
          const gemStatusMap = await invoke<Record<string, GemPreview>>('check_recording_gems_batch', { filenames });

          // Initialize recording states, preserving any existing state (e.g., ongoing transcription)
          setRecordingStates(prev => {
            const newStates: Record<string, RecordingTranscriptionState> = {};
            state.recordings.forEach(recording => {
              newStates[recording.filename] = prev[recording.filename] || {
                transcribing: false,
                hasGem: recording.filename in gemStatusMap,
                savingGem: false,
                gemSaved: false,
              };
            });
            return newStates;
          });
        }
      } catch (error) {
        console.error('Failed to check availability or gem status:', error);
      }
    };

    if (!isLoadingRecordings && state.recordings.length > 0) {
      checkAvailabilityAndGems();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.recordings.length, isLoadingRecordings]);

  // Listen for youtube-video-detected events to show notification badge
  useTauriEvent<YouTubeDetectedEvent>(
    'youtube-video-detected',
    useCallback(() => {
      console.log('[App] youtube-video-detected event received, activeNav:', activeNav);
      if (activeNav !== 'youtube') {
        setYoutubeNotification(true);
      }
    }, [activeNav])
  );

  // Listen for notification clicks to open YouTube section
  useEffect(() => {
    let cleanup: any;
    
    onAction(() => {
      console.log('[App] Notification clicked, opening YouTube section');
      setActiveNav('youtube');
      setYoutubeNotification(false);
    }).then(unlisten => {
      cleanup = unlisten;
    });
    
    return () => {
      if (cleanup && typeof cleanup === 'function') {
        cleanup();
      }
    };
  }, []);

  // Listen for MLX sidecar errors (broken pipe, crashes)
  useTauriEvent<{ error: string }>(
    'mlx-sidecar-error',
    useCallback((event) => {
      console.error('[App] MLX sidecar error:', event.error);
      setToastError(`AI enrichment error: ${event.error}`);
    }, [])
  );

  // Cleanup audio URL when component unmounts or selection changes
  useEffect(() => {
    return () => {
      if (audioUrl) {
        URL.revokeObjectURL(audioUrl);
      }
    };
  }, [audioUrl]);

  /**
   * Handle start recording button click
   * Requirement 8.5: Display inline error for concurrent recording attempts
   */
  const handleStartRecording = async () => {
    await startRecording();
  };

  const handleStopRecording = async () => {
    await stopRecording();
  };

  /**
   * Handle recording selection and WAV conversion for playback
   * Requirements: 4.3, 5.1, 5.2, 5.3
   */
  const handleSelectRecording = async (filename: string) => {
    try {
      // Convert PCM to WAV (Requirement 5.1, 5.2)
      const wavBytes = await invoke<number[]>("convert_to_wav", { filename });
      
      // Create blob URL for playback (Requirement 5.3)
      const blob = new Blob([new Uint8Array(wavBytes)], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);
      
      // Clean up old URL
      if (audioUrl) {
        URL.revokeObjectURL(audioUrl);
      }
      
      setAudioUrl(url);
      selectRecording(filename);
    } catch (error) {
      console.error("Failed to convert recording:", error);
    }
  };

  /**
   * Handle closing the audio player
   */
  const handleClosePlayer = () => {
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      setAudioUrl(null);
    }
    deselectRecording();
  };

  /**
   * Handle recording deletion with confirmation
   * Requirement 6.1: Deletion confirmation prompt
   */
  const handleDeleteRecording = (filename: string) => {
    // Show delete confirmation dialog
    setDeleteTarget(filename);
  };

  /**
   * Confirm deletion after user accepts dialog
   */
  const confirmDelete = async () => {
    if (!deleteTarget) return;
    
    // Close player if deleted recording was selected
    if (state.selectedRecording === deleteTarget) {
      handleClosePlayer();
    }
    
    // Delete the recording (Requirement 6.2, 6.3)
    await deleteRecording(deleteTarget);
    setDeleteTarget(null);
  };

  /**
   * Cancel deletion
   */
  const cancelDelete = () => {
    setDeleteTarget(null);
  };

  /**
   * Handle retry after permission error
   */
  const handleRetryRecording = async () => {
    await retryRecording();
  };
  
  /**
   * Handle transcribe button click
   */
  const handleTranscribeRecording = async (filename: string) => {
    // Update state to show loading
    setRecordingStates(prev => ({
      ...prev,
      [filename]: {
        ...prev[filename],
        transcribing: true,
        transcriptError: undefined,
        gemSaved: false,
      }
    }));
    
    try {
      const result = await invoke<TranscriptResult>('transcribe_recording', { filename });
      
      // Check if gem exists for this recording
      const gemPreview = await invoke<GemPreview | null>('check_recording_gem', { filename });
      
      setRecordingStates(prev => ({
        ...prev,
        [filename]: {
          ...prev[filename],
          transcribing: false,
          transcript: result,
          hasGem: gemPreview !== null,
        }
      }));
    } catch (error) {
      console.error('Transcription failed:', error);
      setRecordingStates(prev => ({
        ...prev,
        [filename]: {
          ...prev[filename],
          transcribing: false,
          transcriptError: String(error),
        }
      }));
    }
  };
  
  /**
   * Handle save/update gem button click
   */
  const handleSaveGem = async (filename: string) => {
    const recordingState = recordingStates[filename];
    if (!recordingState?.transcript) return;
    
    const recording = state.recordings.find(r => r.filename === filename);
    if (!recording) return;
    
    // Update state to show loading
    setRecordingStates(prev => ({
      ...prev,
      [filename]: {
        ...prev[filename],
        savingGem: true,
        gemError: undefined,
        gemSaved: false,
      }
    }));
    
    try {
      await invoke<Gem>('save_recording_gem', {
        filename,
        transcript: recordingState.transcript.transcript,
        language: recordingState.transcript.language,
        createdAt: recording.created_at,
      });
      
      setRecordingStates(prev => ({
        ...prev,
        [filename]: {
          ...prev[filename],
          savingGem: false,
          hasGem: true,
          gemSaved: true,
        }
      }));
      
      // Clear transcript and success indicator after 3 seconds
      setTimeout(() => {
        setRecordingStates(prev => ({
          ...prev,
          [filename]: {
            ...prev[filename],
            gemSaved: false,
            transcript: undefined,
            transcriptError: undefined,
          }
        }));
      }, 3000);
    } catch (error) {
      console.error('Failed to save gem:', error);
      setRecordingStates(prev => ({
        ...prev,
        [filename]: {
          ...prev[filename],
          savingGem: false,
          gemError: String(error),
        }
      }));
    }
  };
  
  /**
   * Handle gem selection from GemsPanel
   */
  const handleGemSelect = (gemId: string | null) => {
    setSelectedGemId(gemId);
  };
  
  /**
   * Handle gem deletion from right panel
   */
  const handleDeleteGem = async () => {
    if (!selectedGemId) return;
    
    try {
      await invoke('delete_gem', { id: selectedGemId });
      setSelectedGemId(null);
      // Trigger gems panel refresh by incrementing key
      setGemsPanelRefreshKey(prev => prev + 1);
    } catch (error) {
      console.error('Failed to delete gem:', error);
      setToastError(`Failed to delete gem: ${error}`);
    }
  };
  
  /**
   * Handle gem transcription from right panel
   */
  const handleTranscribeGem = async () => {
    if (!selectedGemId) return;
    
    try {
      await invoke('transcribe_gem', { id: selectedGemId });
      // Gem will be updated, could trigger a refresh here
    } catch (error) {
      console.error('Failed to transcribe gem:', error);
      setToastError(`Failed to transcribe gem: ${error}`);
    }
  };
  
  /**
   * Handle gem enrichment from right panel
   */
  const handleEnrichGem = async () => {
    if (!selectedGemId) return;
    
    try {
      await invoke('enrich_gem', { id: selectedGemId });
      // Gem will be updated, could trigger a refresh here
    } catch (error) {
      console.error('Failed to enrich gem:', error);
      setToastError(`Failed to enrich gem: ${error}`);
    }
  };
  
  /**
   * Handle navigation change
   */
  const handleNavChange = (nav: ActiveNav) => {
    setActiveNav(nav);
    
    // Clear YouTube notification when navigating to YouTube
    if (nav === 'youtube') {
      setYoutubeNotification(false);
    }
  };
  
  /**
   * Handle left nav collapse toggle
   */
  const handleToggleCollapse = () => {
    setLeftNavCollapsed(!leftNavCollapsed);
  };

  /**
   * Format time in MM:SS format
   * Requirement 4.2: Display duration in MM:SS format
   */
  const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
  };

  return (
    <div className={`app-layout${isResizing ? ' is-resizing' : ''}`}>
      {/* Left Navigation Panel */}
      <LeftNav
        activeNav={activeNav}
        onNavChange={handleNavChange}
        youtubeNotification={youtubeNotification}
        collapsed={leftNavCollapsed}
        onToggleCollapse={handleToggleCollapse}
      />
      
      {/* Center Content Panel */}
      <div className="center-panel">
        {activeNav === 'record' && (
          <>
            {/* Status Display */}
            <div className="status">
              {state.recordingState === "idle" && <p>Ready to record</p>}
              {state.recordingState === "recording" && (
                <p className="recording">Recording... {formatTime(state.elapsedTime)}</p>
              )}
              {state.recordingState === "processing" && <p>Processing...</p>}
            </div>

            {/* Record Button */}
            <div className="button-container">
              {state.recordingState === "idle" && (
                <button
                  className="record-button"
                  onClick={handleStartRecording}
                >
                  ⏺ Start Recording
                </button>
              )}
              {state.recordingState === "recording" && (
                <button
                  className="record-button stop recording"
                  onClick={handleStopRecording}
                >
                  ⏹ Stop Recording
                </button>
              )}
              {state.recordingState === "processing" && (
                <button className="record-button" disabled>
                  <span className="spinner"></span>
                  Processing...
                </button>
              )}
              
              {/* Inline error for concurrent recording attempts */}
              {state.error && state.error.toLowerCase().includes("already") && (
                <div className="inline-error">
                  {state.error}
                </div>
              )}
            </div>

            {/* Error Display - Don't show concurrent recording errors here */}
            {state.error && !state.showPermissionDialog && !state.error.toLowerCase().includes("already") && (
              <div className="error">
                <p>{state.error}</p>
                <button onClick={clearError}>
                  Dismiss
                </button>
              </div>
            )}
          </>
        )}
        
        {activeNav === 'recordings' && (
          <>
            {isLoadingRecordings ? (
              <div className="recordings-section">
                <h2>Recordings</h2>
                <div className="skeleton-loader">
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="skeleton-item">
                      <div className="skeleton-line long"></div>
                      <div className="skeleton-line short"></div>
                    </div>
                  ))}
                </div>
              </div>
            ) : state.recordings.length > 0 ? (
              <div className="recordings-section">
                <h2>Recordings ({state.recordings.length})</h2>
                <div className="recordings-list">
                  {state.recordings.map((recording) => {
                    const recordingState = recordingStates[recording.filename] || {
                      transcribing: false,
                      hasGem: false,
                      savingGem: false,
                      gemSaved: false,
                    };
                    
                    return (
                      <div
                        key={recording.filename}
                        className={`recording-item ${
                          state.selectedRecording === recording.filename ? "selected" : ""
                        }`}
                      >
                        <div
                          className="recording-info"
                          onClick={() => handleSelectRecording(recording.filename)}
                        >
                          <div className="recording-name">
                            {recording.filename}
                            {recordingState.hasGem && <span className="gem-indicator" title="Has gem">💎</span>}
                          </div>
                          <div className="recording-meta">
                            {formatDate(recording.created_at)} • {formatTime(Math.floor(recording.duration_seconds))} • {formatFileSize(recording.size_bytes)}
                          </div>
                        </div>
                        <div className="recording-actions">
                          {aiAvailable && (
                            <button
                              className="transcribe-button"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleTranscribeRecording(recording.filename);
                              }}
                              disabled={recordingState.transcribing}
                              title="Transcribe recording"
                            >
                              {recordingState.transcribing ? "⏳" : "📝"}
                            </button>
                          )}
                          <button
                            className="delete-button"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDeleteRecording(recording.filename);
                            }}
                            title="Delete recording"
                          >
                            🗑️
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            ) : (
              <div className="recordings-section">
                <p className="empty-message">No recordings yet. Start recording to create your first one!</p>
              </div>
            )}
          </>
        )}
        
        {activeNav === 'gems' && (
          <GemsPanel key={gemsPanelRefreshKey} onGemSelect={handleGemSelect} />
        )}
        
        {activeNav === 'youtube' && (
          <YouTubeSection />
        )}
        
        {activeNav === 'browser' && (
          <BrowserTool />
        )}
        
        {activeNav === 'settings' && (
          <Settings />
        )}
      </div>
      
      {/* Resize Handle */}
      {showRightPanel && (
        <div className="resize-handle" onMouseDown={handleResizeMouseDown} />
      )}

      {/* Right Context Panel */}
      <RightPanel
        activeNav={activeNav}
        selectedRecording={state.selectedRecording}
        selectedGemId={selectedGemId}
        recordingState={state.recordingState}
        transcript={state.transcript}
        transcriptionStatus={state.transcriptionStatus}
        transcriptionError={state.transcriptionError}
        audioUrl={audioUrl}
        onClosePlayer={handleClosePlayer}
        recordingStates={recordingStates}
        onTranscribeRecording={handleTranscribeRecording}
        onSaveGem={() => state.selectedRecording && handleSaveGem(state.selectedRecording)}
        onDeleteGem={handleDeleteGem}
        onTranscribeGem={handleTranscribeGem}
        onEnrichGem={handleEnrichGem}
        aiAvailable={aiAvailable}
        recordings={state.recordings}
        currentRecording={state.currentRecording}
        style={{ width: rightPanelWidth }}
      />

      {/* Permission Dialog */}
      {state.showPermissionDialog && (
        <div className="dialog-overlay">
          <div className="dialog">
            <h2>Permission Required</h2>
            <p>
              {state.error || "JarvisApp needs Screen Recording and Microphone permissions to capture audio."}
            </p>
            <div className="dialog-buttons">
              <button onClick={openSystemSettings}>
                Open System Settings
              </button>
              <button onClick={handleRetryRecording}>
                Retry
              </button>
              <button onClick={clearError}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      <DeleteConfirmDialog
        visible={deleteTarget !== null}
        recordingName={deleteTarget ?? ""}
        onConfirm={confirmDelete}
        onCancel={cancelDelete}
      />

      {/* Error Toast for MLX sidecar crashes and other runtime errors */}
      <ErrorToast
        message={toastError}
        onClose={() => setToastError(null)}
      />
    </div>
  );
}

export default App;
