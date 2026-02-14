
import { RecordingMetadata } from '../state/types';
import { RecordingRow } from './RecordingRow';

interface RecordingsListProps {
  recordings: RecordingMetadata[];
  selectedRecording: string | null;
  onSelect: (filename: string) => void;
  onDelete: (filename: string) => void;
}

export function RecordingsList({
  recordings,
  selectedRecording,
  onSelect,
  onDelete,
}: RecordingsListProps) {
  if (recordings.length === 0) {
    return (
      <div className="recordings-list empty">
        <p className="empty-message">No recordings yet</p>
      </div>
    );
  }

  return (
    <div className="recordings-list">
      {recordings.map((recording) => (
        <RecordingRow
          key={recording.filename}
          recording={recording}
          selected={selectedRecording === recording.filename}
          onSelect={() => onSelect(recording.filename)}
          onDelete={() => onDelete(recording.filename)}
        />
      ))}
    </div>
  );
}
