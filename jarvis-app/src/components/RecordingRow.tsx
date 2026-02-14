
import { RecordingMetadata } from '../state/types';
import { formatDuration, formatFileSize, formatTimestamp } from '../utils/formatters';

interface RecordingRowProps {
  recording: RecordingMetadata;
  selected: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

export function RecordingRow({ recording, selected, onSelect, onDelete }: RecordingRowProps) {
  return (
    <div
      className={`recording-row ${selected ? 'selected' : ''}`}
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      <div className="recording-info">
        <div className="recording-timestamp">
          {formatTimestamp(recording.created_at)}
        </div>
        <div className="recording-details">
          <span className="recording-duration">
            {formatDuration(recording.duration_seconds)}
          </span>
          <span className="recording-size">
            {formatFileSize(recording.size_bytes)}
          </span>
        </div>
      </div>
      <button
        className="delete-button"
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        aria-label={`Delete recording ${recording.filename}`}
      >
        <svg viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z" />
        </svg>
      </button>
    </div>
  );
}
