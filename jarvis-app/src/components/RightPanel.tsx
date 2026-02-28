import { useState } from 'react';
import { TranscriptDisplay } from './TranscriptDisplay';
// import { CoPilotPanel } from './CoPilotPanel'; // Replaced by CoPilotCardStack
import { CoPilotCardStack } from './CoPilotCardStack';
import RecordingDetailPanel from './RecordingDetailPanel';
import GemDetailPanel from './GemDetailPanel';
import { RecordingMetadata, TranscriptionSegment, RecordingTranscriptionState, CoPilotState, CoPilotStatus } from '../state/types';

type ActiveNav = 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings';

interface RightPanelProps {
  activeNav: ActiveNav;
  selectedRecording: string | null;
  selectedGemId: string | null;
  recordingState: 'idle' | 'recording' | 'processing';
  transcript: TranscriptionSegment[];
  transcriptionStatus: 'idle' | 'active' | 'error' | 'disabled';
  transcriptionError: string | null;
  audioUrl: string | null;
  onClosePlayer: () => void;
  recordingStates: Record<string, RecordingTranscriptionState>;
  onTranscribeRecording: (filename: string) => void;
  onSaveGem: () => void;
  onDeleteGem: () => void;
  onTranscribeGem: () => void;
  onEnrichGem: () => void;
  aiAvailable: boolean;
  recordings: RecordingMetadata[];
  currentRecording: string | null;
  copilotEnabled: boolean;
  copilotStatus: CoPilotStatus;
  copilotState: CoPilotState | null;
  copilotError: string | null;
  onDismissCopilotQuestion: (index: number) => void;
  style?: React.CSSProperties;
}

export default function RightPanel({
  activeNav,
  selectedRecording,
  selectedGemId,
  recordingState,
  transcript,
  transcriptionStatus,
  transcriptionError,
  audioUrl,
  onClosePlayer,
  recordingStates,
  onTranscribeRecording,
  onSaveGem,
  onDeleteGem,
  onTranscribeGem,
  onEnrichGem,
  aiAvailable,
  recordings,
  currentRecording,
  copilotEnabled,
  copilotStatus,
  copilotState,
  copilotError,
  onDismissCopilotQuestion,
  style
}: RightPanelProps) {
  const [activeTab, setActiveTab] = useState<'transcript' | 'copilot'>('transcript');
  const [hasSeenCopilotUpdate, setHasSeenCopilotUpdate] = useState(false);

  // Show notification dot when copilot has new data and user is on transcript tab
  const showNotificationDot = copilotEnabled && 
    activeTab === 'transcript' && 
    copilotState && 
    copilotState.cycle_metadata.cycle_number > 0 &&
    !hasSeenCopilotUpdate;

  // Clear notification when user switches to copilot tab
  const handleTabChange = (tab: 'transcript' | 'copilot') => {
    setActiveTab(tab);
    if (tab === 'copilot') {
      setHasSeenCopilotUpdate(true);
    }
  };

  // Reset notification when copilot state updates
  if (copilotState && copilotState.cycle_metadata.cycle_number > 0 && activeTab === 'transcript') {
    if (hasSeenCopilotUpdate) {
      setHasSeenCopilotUpdate(false);
    }
  }
  // Record nav: show live transcript when recording or after recording completes
  if (activeNav === 'record') {
    const isRecording = recordingState === 'recording';
    const hasTranscript = transcript.length > 0;
    const recordingCompleted = !isRecording && hasTranscript;
    
    // Keep Co-Pilot tab visible if copilot is enabled AND (recording OR has copilot data)
    const hasCopilotData = copilotState && copilotState.cycle_metadata.cycle_number > 0;
    const showCopilotTab = (copilotEnabled && isRecording) || !!hasCopilotData;

    if (isRecording || recordingCompleted) {
      // Show tabs when copilot is enabled and recording or has data
      if (showCopilotTab) {
        return (
          <div className="right-panel" style={style}>
            <div className="record-tabs-view">
              <div className="tab-buttons">
                <button
                  className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
                  onClick={() => handleTabChange('transcript')}
                >
                  Transcript
                </button>
                <button
                  className={`tab-button ${activeTab === 'copilot' ? 'active' : ''}`}
                  onClick={() => handleTabChange('copilot')}
                >
                  Co-Pilot
                  {showNotificationDot && <span className="notification-dot" />}
                </button>
              </div>
              <div className="tab-content">
                {activeTab === 'transcript' ? (
                  <TranscriptDisplay
                    transcript={transcript}
                    status={transcriptionStatus}
                    error={transcriptionError}
                    recordingFilename={currentRecording}
                  />
                ) : (
                  <CoPilotCardStack
                    state={copilotState}
                    status={copilotStatus}
                    error={copilotError}
                    recordingState={recordingState}
                    cycleInterval={60} // TODO: Wire to settings.copilot.cycle_interval when settings state is available
                    onDismissQuestion={onDismissCopilotQuestion}
                  />
                )}
              </div>
            </div>
          </div>
        );
      }

      // Show transcript only when copilot is disabled
      return (
        <div className="right-panel" style={style}>
          <TranscriptDisplay
            transcript={transcript}
            status={transcriptionStatus}
            error={transcriptionError}
            recordingFilename={currentRecording}
          />
        </div>
      );
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Start recording to see live transcript
        </div>
      </div>
    );
  }

  // Recordings nav: show recording detail panel when a recording is selected
  if (activeNav === 'recordings') {
    if (selectedRecording && audioUrl) {
      const recording = recordings.find(r => r.filename === selectedRecording);
      if (recording) {
        const recState = recordingStates[selectedRecording] || {
          transcribing: false,
          hasGem: false,
          savingGem: false,
          gemSaved: false
        };

        return (
          <div className="right-panel" style={style}>
            <RecordingDetailPanel
              recording={recording}
              audioUrl={audioUrl}
              onClose={onClosePlayer}
              recordingState={recState}
              onTranscribe={() => onTranscribeRecording(selectedRecording)}
              onSaveGem={onSaveGem}
              aiAvailable={aiAvailable}
            />
          </div>
        );
      }
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Select a recording to play or transcribe
        </div>
      </div>
    );
  }

  // Gems nav: show gem detail panel when a gem is selected
  if (activeNav === 'gems') {
    if (selectedGemId) {
      return (
        <div className="right-panel" style={style}>
          <GemDetailPanel
            gemId={selectedGemId}
            onDelete={onDeleteGem}
            onTranscribe={onTranscribeGem}
            onEnrich={onEnrichGem}
            aiAvailable={aiAvailable}
          />
        </div>
      );
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Select a gem to view details
        </div>
      </div>
    );
  }

  // For youtube, browser, settings: return null (no right panel content)
  return null;
}
