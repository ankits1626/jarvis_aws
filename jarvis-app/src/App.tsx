import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onAction } from "@tauri-apps/plugin-notification";
import { useRecording } from "./hooks/useRecording";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { DeleteConfirmDialog } from "./components/DeleteConfirmDialog";
import { TranscriptDisplay } from "./components/TranscriptDisplay";
import { ErrorToast } from "./components/ErrorToast";
import { Settings } from "./components/Settings";
import { YouTubeSection } from "./components/YouTubeSection";
import { BrowserTool } from "./components/BrowserTool";
import { GemsPanel } from "./components/GemsPanel";
import type { YouTubeDetectedEvent, TranscriptResult, RecordingTranscriptionState, GemPreview, AvailabilityResult, Gem } from "./state/types";
import "./App.css";

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
  const [showSettings, setShowSettings] = useState(false);
  const [showHamburgerMenu, setShowHamburgerMenu] = useState(false);
  const [youtubeNotification, setYoutubeNotification] = useState(false);
  const [showYouTube, setShowYouTube] = useState(false);
  const [showBrowserTool, setShowBrowserTool] = useState(false);
  const [showGems, setShowGems] = useState(false);
  const [toastError, setToastError] = useState<string | null>(null);
  
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

  // Close hamburger menu when clicking outside
  useEffect(() => {
    if (!showHamburgerMenu) return;
    
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('.hamburger-button') && !target.closest('.hamburger-menu')) {
        setShowHamburgerMenu(false);
      }
    };
    
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [showHamburgerMenu]);

  // Listen for youtube-video-detected events to show notification badge
  useTauriEvent<YouTubeDetectedEvent>(
    'youtube-video-detected',
    useCallback(() => {
      console.log('[App] youtube-video-detected event received, showYouTube:', showYouTube);
      if (!showYouTube) {
        setYoutubeNotification(true);
      }
    }, [showYouTube])
  );

  // Listen for notification clicks to open YouTube section
  useEffect(() => {
    let cleanup: any;
    
    onAction(() => {
      console.log('[App] Notification clicked, opening YouTube section');
      setShowYouTube(true);
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
   * Handle opening YouTube section
   */
  const handleOpenYouTube = () => {
    setShowYouTube(true);
    setShowHamburgerMenu(false);
    setYoutubeNotification(false);
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
    <div className="app">
      <div className="container">
        <div className="header">
          <h1>JarvisApp</h1>
          <div className="header-buttons">
            <button
              className="hamburger-button"
              onClick={() => setShowHamburgerMenu(!showHamburgerMenu)}
              title="Menu"
            >
              ☰
              {youtubeNotification && <span className="notification-badge" />}
            </button>
            <button
              className="settings-button"
              onClick={() => setShowSettings(true)}
              title="Settings"
            >
              ⚙️
            </button>
          </div>
          
          {/* Hamburger dropdown menu */}
          {showHamburgerMenu && (
            <div className="hamburger-menu">
              <button
                className="hamburger-menu-item"
                onClick={handleOpenYouTube}
              >
                📹 YouTube
              </button>
              <button
                className="hamburger-menu-item"
                onClick={() => { setShowBrowserTool(true); setShowHamburgerMenu(false); }}
              >
                🌐 Browser
              </button>
              <button
                className="hamburger-menu-item"
                onClick={() => { setShowGems(true); setShowHamburgerMenu(false); }}
              >
                💎 Gems
              </button>
            </div>
          )}
        </div>
        
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
          
          {/* Requirement 8.5: Inline error for concurrent recording attempts */}
          {state.error && state.error.toLowerCase().includes("already") && (
            <div className="inline-error">
              {state.error}
            </div>
          )}
        </div>

        {/* Recordings List - Requirements 4.1, 4.2, 4.3, 4.4 */}
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
                  <div key={recording.filename} className="recording-item-container">
                    <div
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
                    
                    {/* Transcript display */}
                    {recordingState.transcript && (
                      <div className="transcript-container">
                        <div className="transcript-header">
                          <span className="transcript-label">Transcript ({recordingState.transcript.language})</span>
                        </div>
                        <div className="transcript-text">
                          {recordingState.transcript.transcript}
                        </div>
                        
                        {/* Save/Update Gem button */}
                        <div className="gem-actions">
                          <button
                            className="save-gem-button"
                            onClick={() => handleSaveGem(recording.filename)}
                            disabled={recordingState.savingGem}
                          >
                            {recordingState.savingGem ? (
                              <>
                                <span className="spinner"></span>
                                Saving...
                              </>
                            ) : recordingState.gemSaved ? (
                              "✓ Saved!"
                            ) : recordingState.hasGem ? (
                              "Update Gem"
                            ) : (
                              "Save as Gem"
                            )}
                          </button>
                          {recordingState.gemError && (
                            <div className="gem-error">{recordingState.gemError}</div>
                          )}
                        </div>
                      </div>
                    )}
                    
                    {/* Transcription error */}
                    {recordingState.transcriptError && (
                      <div className="transcript-error">
                        Error: {recordingState.transcriptError}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ) : (
          // Requirement 4.4: Display message when list is empty
          <div className="recordings-section">
            <p className="empty-message">No recordings yet. Start recording to create your first one!</p>
          </div>
        )}

        {/* Audio Player - Requirements 5.3, 5.4, 5.5, 5.6 */}
        {state.selectedRecording && audioUrl && (
          <div className="audio-player">
            <div className="player-header">
              <h3>Playing: {state.selectedRecording}</h3>
              <button className="close-button" onClick={handleClosePlayer}>
                ✕
              </button>
            </div>
            <audio
              controls
              src={audioUrl}
              autoPlay
              onEnded={() => {
                // Requirement 5.6: Reset to beginning on completion
                const audio = document.querySelector("audio");
                if (audio) audio.currentTime = 0;
              }}
            />
          </div>
        )}

        {/* Transcript Display - Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 9.6 */}
        <TranscriptDisplay
          transcript={state.transcript}
          status={state.transcriptionStatus}
          error={state.transcriptionError}
          recordingFilename={state.currentRecording || (state.recordings[0]?.filename ?? null)}
        />

        {/* Error Display - Requirement 8.4 */}
        {/* Don't show concurrent recording errors here - they appear inline (Requirement 8.5) */}
        {state.error && !state.showPermissionDialog && !state.error.toLowerCase().includes("already") && (
          <div className="error">
            <p>{state.error}</p>
            <button onClick={clearError}>
              Dismiss
            </button>
          </div>
        )}

        {/* Permission Dialog - Requirement 8.1 */}
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

        {/* Delete Confirmation Dialog - Requirement 6.1 */}
        <DeleteConfirmDialog
          visible={deleteTarget !== null}
          recordingName={deleteTarget ?? ""}
          onConfirm={confirmDelete}
          onCancel={cancelDelete}
        />

        {/* Settings Panel */}
        {showSettings && (
          <div className="dialog-overlay">
            <Settings onClose={() => setShowSettings(false)} />
          </div>
        )}

        {/* YouTube Section */}
        {showYouTube && (
          <div className="dialog-overlay" onClick={(e) => {
            if (e.target === e.currentTarget) setShowYouTube(false);
          }}>
            <YouTubeSection onClose={() => setShowYouTube(false)} />
          </div>
        )}

        {/* Browser Tool */}
        {showBrowserTool && (
          <div className="dialog-overlay" onClick={(e) => {
            if (e.target === e.currentTarget) setShowBrowserTool(false);
          }}>
            <BrowserTool onClose={() => setShowBrowserTool(false)} />
          </div>
        )}

        {/* Gems Panel */}
        {showGems && (
          <div className="dialog-overlay" onClick={(e) => {
            if (e.target === e.currentTarget) setShowGems(false);
          }}>
            <GemsPanel onClose={() => setShowGems(false)} />
          </div>
        )}

        {/* Error Toast for MLX sidecar crashes and other runtime errors */}
        <ErrorToast
          message={toastError}
          onClose={() => setToastError(null)}
        />
      </div>
    </div>
  );
}

export default App;
