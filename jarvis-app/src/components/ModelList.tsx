import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ModelInfo, Settings } from '../state/types';

interface ModelListProps {
  models: ModelInfo[];
  selectedModel: string;
  onModelSelected: (modelName: string) => void;
  onDownloadStarted?: (modelName: string) => void;
  downloadCommand?: string;
  cancelCommand?: string;
  deleteCommand?: string;
  settingsField?: 'whisper_model' | 'whisperkit_model';
}

const TIER_LABELS: Record<string, { label: string; color: string }> = {
  basic: { label: 'Basic', color: '#888' },
  good: { label: 'Good', color: '#2196F3' },
  great: { label: 'Great', color: '#4CAF50' },
  best: { label: 'Best', color: '#FF9800' },
};

export function ModelList({ 
  models, 
  selectedModel, 
  onModelSelected, 
  onDownloadStarted,
  downloadCommand = 'download_model',
  cancelCommand = 'cancel_download',
  deleteCommand = 'delete_model',
  settingsField = 'whisper_model',
}: ModelListProps) {
  const [deletingModel, setDeletingModel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const handleDownload = async (modelName: string) => {
    try {
      setError(null);
      await invoke(downloadCommand, { modelName });
      onDownloadStarted?.(modelName);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancel = async (modelName: string) => {
    if (!cancelCommand) return;
    try {
      setError(null);
      await invoke(cancelCommand, { modelName });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDelete = async (modelName: string) => {
    if (!deleteCommand) return;
    try {
      setError(null);
      await invoke(deleteCommand, { modelName });
      setDeletingModel(null);
      onModelSelected(modelName);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setDeletingModel(null);
    }
  };

  const handleSelect = async (modelName: string) => {
    try {
      setError(null);
      // Get current settings first
      const settings = await invoke<Settings>('get_settings');
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          [settingsField]: modelName,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
      onModelSelected(modelName);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const confirmDelete = (modelName: string) => {
    if (modelName === selectedModel) {
      if (window.confirm(
        `"${modelName}" is currently selected. Deleting it will require you to select a different model. Continue?`
      )) {
        setDeletingModel(modelName);
      }
    } else {
      setDeletingModel(modelName);
    }
  };

  return (
    <div className="model-list">
      {error && (
        <div className="model-list-error">
          Error: {error}
          <button onClick={() => setError(null)} className="dismiss-button">×</button>
        </div>
      )}

      {models.map((model) => {
        const isSelected = model.filename === selectedModel;
        const isDeleting = deletingModel === model.filename;
        const tier = TIER_LABELS[model.quality_tier] || TIER_LABELS.basic;

        return (
          <div
            key={model.filename}
            className={`model-item ${isSelected ? 'selected' : ''}`}
          >
            <div className="model-info">
              <div className="model-header">
                <span className="model-name">{model.display_name}</span>
                <span className="model-tier" style={{ color: tier.color }}>{tier.label}</span>
                <span className="model-size-estimate">{model.size_estimate}</span>
              </div>
              <div className="model-description">{model.description}</div>
              <div className="model-filename">{model.filename}</div>

              {model.status.type === 'downloaded' && (
                <span className="model-size">{formatBytes(model.status.size_bytes)}</span>
              )}

              {model.status.type === 'error' && (
                <span className="model-error">Error: {model.status.message}</span>
              )}
            </div>

            <div className="model-actions">
              {model.status.type === 'not_downloaded' && (
                <button
                  onClick={() => handleDownload(model.filename)}
                  className="download-button"
                >
                  Download
                </button>
              )}

              {model.status.type === 'downloading' && (
                <>
                  <div className="progress-container">
                    <div
                      className="progress-bar"
                      style={{ width: `${model.status.progress}%` }}
                    />
                    <span className="progress-text">
                      {model.status.progress.toFixed(0)}%
                    </span>
                  </div>
                  {cancelCommand && (
                    <button
                      onClick={() => handleCancel(model.filename)}
                      className="cancel-button"
                    >
                      Cancel
                    </button>
                  )}
                </>
              )}

              {model.status.type === 'downloaded' && !isDeleting && (
                <>
                  <button
                    onClick={() => handleSelect(model.filename)}
                    disabled={isSelected}
                    className="select-button"
                  >
                    {isSelected ? 'Selected' : 'Select'}
                  </button>
                  {deleteCommand && (
                    <button
                      onClick={() => confirmDelete(model.filename)}
                      className="delete-button"
                    >
                      Delete
                    </button>
                  )}
                </>
              )}

              {model.status.type === 'downloaded' && isDeleting && (
                <div className="delete-confirm">
                  <span>Delete this model?</span>
                  <button
                    onClick={() => handleDelete(model.filename)}
                    className="confirm-delete-button"
                  >
                    Yes
                  </button>
                  <button
                    onClick={() => setDeletingModel(null)}
                    className="cancel-delete-button"
                  >
                    No
                  </button>
                </div>
              )}

              {model.status.type === 'error' && (
                <button
                  onClick={() => handleDownload(model.filename)}
                  className="retry-button"
                >
                  Retry
                </button>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
