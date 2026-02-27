import { TranscriptDisplay } from './TranscriptDisplay';
import RecordingDetailPanel from './RecordingDetailPanel';
import GemDetailPanel from './GemDetailPanel';
import { RecordingMetadata, TranscriptionSegment, RecordingTranscriptionState } from '../state/types';

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
  style
}: RightPanelProps) {
  // Record nav: show live transcript when recording or after recording completes
  if (activeNav === 'record') {
    const isRecording = recordingState === 'recording';
    const hasTranscript = transcript.length > 0;
    const recordingCompleted = !isRecording && hasTranscript;

    if (isRecording || recordingCompleted) {
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
