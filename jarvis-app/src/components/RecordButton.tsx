

type RecordingState = 'idle' | 'recording' | 'processing';

interface RecordButtonProps {
  state: RecordingState;
  onStart: () => void;
  onStop: () => void;
}

export function RecordButton({ state, onStart, onStop }: RecordButtonProps) {
  const handleClick = () => {
    if (state === 'recording') {
      onStop();
    } else if (state === 'idle') {
      onStart();
    }
  };

  const isDisabled = state === 'processing';

  return (
    <button
      className={`record-button ${state}`}
      onClick={handleClick}
      disabled={isDisabled}
      aria-label={state === 'recording' ? 'Stop recording' : 'Start recording'}
    >
      {state === 'processing' && (
        <div className="spinner" />
      )}
      {state === 'idle' && (
        <svg className="record-icon" viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="12" r="8" />
        </svg>
      )}
      {state === 'recording' && (
        <svg className="stop-icon" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="6" width="12" height="12" />
        </svg>
      )}
    </button>
  );
}
