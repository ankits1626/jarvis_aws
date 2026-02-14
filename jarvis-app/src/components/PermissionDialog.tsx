

interface PermissionDialogProps {
  visible: boolean;
  message: string;
  onOpenSettings: () => void;
  onRetry: () => void;
  onClose: () => void;
}

export function PermissionDialog({
  visible,
  message,
  onOpenSettings,
  onRetry,
  onClose,
}: PermissionDialogProps) {
  if (!visible) {
    return null;
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog permission-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2>Permission Required</h2>
          <button
            className="close-button"
            onClick={onClose}
            aria-label="Close dialog"
          >
            <svg viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
            </svg>
          </button>
        </div>

        <div className="dialog-content">
          <p className="permission-message">{message}</p>
          <p className="permission-guidance">
            JarvisApp needs Screen Recording and Microphone permissions to capture audio.
            Please grant these permissions in System Settings and try again.
          </p>
        </div>

        <div className="dialog-actions">
          <button
            className="button button-primary"
            onClick={onOpenSettings}
          >
            Open System Settings
          </button>
          <button
            className="button button-secondary"
            onClick={onRetry}
          >
            Retry
          </button>
          <button
            className="button button-tertiary"
            onClick={onClose}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
