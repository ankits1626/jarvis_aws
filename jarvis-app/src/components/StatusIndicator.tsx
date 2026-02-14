

type RecordingState = 'idle' | 'recording' | 'processing';

interface StatusIndicatorProps {
  state: RecordingState;
  elapsedTime: number;
}

function formatElapsedTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

export function StatusIndicator({ state, elapsedTime }: StatusIndicatorProps) {
  const getStatusText = () => {
    switch (state) {
      case 'idle':
        return 'Idle';
      case 'recording':
        return 'Recording...';
      case 'processing':
        return 'Processing...';
    }
  };

  return (
    <div className={`status-indicator ${state}`}>
      <span className="status-text">{getStatusText()}</span>
      {state === 'recording' && (
        <span className="elapsed-time">{formatElapsedTime(elapsedTime)}</span>
      )}
    </div>
  );
}
