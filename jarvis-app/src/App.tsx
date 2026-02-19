import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onAction } from "@tauri-apps/plugin-notification";
import { useRecording } from "./hooks/useRecording";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { DeleteConfirmDialog } from "./components/DeleteConfirmDialog";
import { TranscriptDisplay } from "./components/TranscriptDisplay";
import { Settings } from "./components/Settings";
import { YouTubeSection } from "./components/YouTubeSection";
import type { YouTubeDetectedEvent } from "./state/types";
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

  // Load recordings on mount (Requirement 1.2)
  useEffect(() => {
    const loadRecordings = async () => {
      setIsLoadingRecordings(true);
      await refreshRecordings();
      setIsLoadingRecordings(false);
    };
    loadRecordings();
  }, [refreshRecordings]);

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
              {state.recordings.map((recording) => (
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
                    <div className="recording-name">{recording.filename}</div>
                    <div className="recording-meta">
                      {formatDate(recording.created_at)} • {formatTime(Math.floor(recording.duration_seconds))} • {formatFileSize(recording.size_bytes)}
                    </div>
                  </div>
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
              ))}
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
      </div>
    </div>
  );
}

export default App;
