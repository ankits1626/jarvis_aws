

interface DeleteConfirmDialogProps {
  visible: boolean;
  recordingName: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteConfirmDialog({
  visible,
  recordingName,
  onConfirm,
  onCancel,
}: DeleteConfirmDialogProps) {
  if (!visible) {
    return null;
  }

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div className="dialog delete-confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2>Delete Recording</h2>
        </div>

        <div className="dialog-content">
          <p>Are you sure you want to delete this recording?</p>
          <p className="recording-name">{recordingName}</p>
          <p className="warning-text">This action cannot be undone.</p>
        </div>

        <div className="dialog-actions">
          <button
            className="button button-danger"
            onClick={onConfirm}
          >
            Delete
          </button>
          <button
            className="button button-secondary"
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
