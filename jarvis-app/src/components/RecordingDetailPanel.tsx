import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RecordingMetadata, RecordingTranscriptionState, ProjectPreview } from '../state/types';

function AddToProjectPicker({ gemId }: { gemId: string }) {
  const [projects, setProjects] = useState<ProjectPreview[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState('');
  const [adding, setAdding] = useState(false);
  const [addedTo, setAddedTo] = useState<string | null>(null);

  useEffect(() => {
    invoke<ProjectPreview[]>('list_projects')
      .then(list => setProjects(list.filter(p => p.status === 'active')))
      .catch(() => {});
  }, []);

  const handleAdd = async () => {
    if (!selectedProjectId) return;
    setAdding(true);
    try {
      await invoke('add_gems_to_project', { projectId: selectedProjectId, gemIds: [gemId] });
      const project = projects.find(p => p.id === selectedProjectId);
      setAddedTo(project?.title || 'project');
    } catch (err) {
      console.error('Failed to add gem to project:', err);
    } finally {
      setAdding(false);
    }
  };

  if (projects.length === 0) return null;

  if (addedTo) {
    return <div className="added-to-project-notice">Added to {addedTo}</div>;
  }

  return (
    <div className="gist-add-to-project">
      <select
        value={selectedProjectId}
        onChange={(e) => setSelectedProjectId(e.target.value)}
        className="project-select"
      >
        <option value="">Add to project...</option>
        {projects.map(p => (
          <option key={p.id} value={p.id}>{p.title}</option>
        ))}
      </select>
      <button
        onClick={handleAdd}
        disabled={!selectedProjectId || adding}
        className="add-to-project-button"
      >
        {adding ? 'Adding...' : 'Add'}
      </button>
    </div>
  );
}

interface RecordingDetailPanelProps {
  recording: RecordingMetadata;
  audioUrl: string;
  onClose: () => void;
  recordingState: RecordingTranscriptionState;
  onTranscribe: () => void;
  onSaveGem: () => void;
  onStartChat: () => void;
  aiAvailable: boolean;
}

export default function RecordingDetailPanel({
  recording,
  audioUrl,
  onClose,
  recordingState,
  onTranscribe,
  onSaveGem,
  onStartChat,
  aiAvailable
}: RecordingDetailPanelProps) {
  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const formatSize = (bytes: number) => {
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(2)} MB`;
  };

  return (
    <div className="recording-detail-panel">
      <div className="recording-detail-header">
        <h3>Recording Details</h3>
        <button className="close-button" onClick={onClose} title="Close">
          ✕
        </button>
      </div>

      <div className="recording-detail-metadata">
        <div className="metadata-item">
          <span className="metadata-label">Filename:</span>
          <span className="metadata-value">{recording.filename}</span>
        </div>
        <div className="metadata-item">
          <span className="metadata-label">Date:</span>
          <span className="metadata-value">{formatDate(recording.created_at)}</span>
        </div>
        <div className="metadata-item">
          <span className="metadata-label">Duration:</span>
          <span className="metadata-value">{formatDuration(recording.duration_seconds)}</span>
        </div>
        <div className="metadata-item">
          <span className="metadata-label">Size:</span>
          <span className="metadata-value">{formatSize(recording.size_bytes)}</span>
        </div>
      </div>

      <div className="recording-detail-audio">
        <audio controls src={audioUrl} className="audio-player">
          Your browser does not support the audio element.
        </audio>
      </div>

      {aiAvailable && (
        <div className="transcription-controls">
          <button
            onClick={onTranscribe}
            disabled={recordingState.transcribing}
            className="transcribe-button"
          >
            {recordingState.transcribing ? 'Transcribing...' : 'Transcribe'}
          </button>
          <button
            onClick={onStartChat}
            disabled={!aiAvailable}
            className="chat-button"
            title={!aiAvailable ? 'No model loaded' : 'Chat with this recording'}
          >
            Chat
          </button>
        </div>
      )}

      {recordingState.transcript && (
        <div className="transcript-section">
          <div className="transcript-header">
            <h4>Transcript</h4>
            {recordingState.transcript.language && (
              <span className="language-indicator">
                {recordingState.transcript.language.toUpperCase()}
              </span>
            )}
          </div>
          <div className="transcript-text">
            {recordingState.transcript.transcript}
          </div>
          <div className="transcript-actions">
            <button
              onClick={onSaveGem}
              disabled={recordingState.savingGem}
              className="save-gem-button"
            >
              {recordingState.hasGem ? 'Update Gem' : 'Save as Gem'}
            </button>
            {recordingState.hasGem && (
              <span className="gem-indicator">💎 Saved as Gem</span>
            )}
          </div>
          {recordingState.gemId && (
            <AddToProjectPicker gemId={recordingState.gemId} />
          )}
        </div>
      )}

      {recordingState.transcriptError && (
        <div className="error-message">
          {recordingState.transcriptError}
        </div>
      )}
    </div>
  );
}
