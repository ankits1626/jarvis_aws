import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ModelList } from './ModelList';
import type {
  Settings,
  ModelInfo,
  ModelProgressEvent,
  ModelDownloadCompleteEvent,
  ModelDownloadErrorEvent,
  SettingsChangedEvent,
} from '../state/types';

interface SettingsProps {
  onClose: () => void;
}

export function Settings({ onClose }: SettingsProps) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load settings and models on mount
  useEffect(() => {
    const loadData = async () => {
      try {
        setLoading(true);
        const [settingsData, modelsData] = await Promise.all([
          invoke<Settings>('get_settings'),
          invoke<ModelInfo[]>('list_models'),
        ]);
        setSettings(settingsData);
        setModels(modelsData);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, []);

  // Listen for settings-changed events
  useEffect(() => {
    const unlisten = listen<SettingsChangedEvent>('settings-changed', (event) => {
      setSettings(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for model-download-progress events
  useEffect(() => {
    const unlisten = listen<ModelProgressEvent>('model-download-progress', (event) => {
      setModels((prevModels) =>
        prevModels.map((model) =>
          model.filename === event.payload.model_name
            ? {
                ...model,
                status: { type: 'downloading', progress: event.payload.progress },
              }
            : model
        )
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for model-download-complete events
  useEffect(() => {
    const unlisten = listen<ModelDownloadCompleteEvent>('model-download-complete', async () => {
      // Refresh model list to get updated status
      try {
        const modelsData = await invoke<ModelInfo[]>('list_models');
        setModels(modelsData);
      } catch (err) {
        console.error('Failed to refresh models:', err);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for model-download-error events
  useEffect(() => {
    const unlisten = listen<ModelDownloadErrorEvent>('model-download-error', (event) => {
      setModels((prevModels) =>
        prevModels.map((model) =>
          model.filename === event.payload.model_name
            ? {
                ...model,
                status: { type: 'error', message: event.payload.error },
              }
            : model
        )
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (loading) {
    return (
      <div className="settings-panel">
        <div className="settings-header">
          <h2>Settings</h2>
          <button onClick={onClose} className="close-button">×</button>
        </div>
        <div className="settings-content">
          <div className="loading">Loading settings...</div>
        </div>
      </div>
    );
  }

  if (error || !settings) {
    return (
      <div className="settings-panel">
        <div className="settings-header">
          <h2>Settings</h2>
          <button onClick={onClose} className="close-button">×</button>
        </div>
        <div className="settings-content">
          <div className="error">Error: {error || 'Failed to load settings'}</div>
        </div>
      </div>
    );
  }

  const handleVadEnabledChange = async (enabled: boolean) => {
    try {
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          vad_enabled: enabled,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleVadThresholdChange = async (threshold: number) => {
    try {
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          vad_threshold: threshold,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleVoskEnabledChange = async (enabled: boolean) => {
    try {
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          vosk_enabled: enabled,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>Settings</h2>
        <button onClick={onClose} className="close-button">×</button>
      </div>
      <div className="settings-content">
        <section className="settings-section">
          <h3>Voice Activity Detection (VAD)</h3>
          <div className="setting-row">
            <label htmlFor="vad-enabled">
              <input
                type="checkbox"
                id="vad-enabled"
                checked={settings.transcription.vad_enabled}
                onChange={(e) => handleVadEnabledChange(e.target.checked)}
              />
              Enable VAD
            </label>
          </div>
          <div className="setting-row">
            <label htmlFor="vad-threshold">
              VAD Threshold: {Math.round(settings.transcription.vad_threshold * 100)}%
            </label>
            <input
              type="range"
              id="vad-threshold"
              min="0"
              max="1"
              step="0.05"
              value={settings.transcription.vad_threshold}
              disabled={!settings.transcription.vad_enabled}
              onChange={(e) => handleVadThresholdChange(parseFloat(e.target.value))}
            />
          </div>
        </section>

        <section className="settings-section">
          <h3>Vosk (Instant Partials)</h3>
          <div className="setting-row">
            <label htmlFor="vosk-enabled">
              <input
                type="checkbox"
                id="vosk-enabled"
                checked={settings.transcription.vosk_enabled}
                onChange={(e) => handleVoskEnabledChange(e.target.checked)}
              />
              Enable Vosk
            </label>
          </div>
        </section>

        <section className="settings-section">
          <h3>Whisper (Accurate Finals)</h3>
          <div className="setting-row">
            <label htmlFor="whisper-enabled">
              <input
                type="checkbox"
                id="whisper-enabled"
                checked={settings.transcription.whisper_enabled}
                disabled={true}
              />
              Enable Whisper
            </label>
            <p className="setting-info">Whisper is required and cannot be disabled</p>
          </div>
        </section>

        <section className="settings-section">
          <h3>Whisper Models</h3>
          <ModelList
            models={models}
            selectedModel={settings.transcription.whisper_model}
            onModelSelected={async () => {
              // Refresh model list after selection or deletion
              try {
                const modelsData = await invoke<ModelInfo[]>('list_models');
                setModels(modelsData);
              } catch (err) {
                console.error('Failed to refresh models:', err);
              }
            }}
            onDownloadStarted={(modelName) => {
              // Optimistic UI update: show downloading state immediately
              setModels((prev) =>
                prev.map((m) =>
                  m.filename === modelName
                    ? { ...m, status: { type: 'downloading', progress: 0 } }
                    : m
                )
              );
            }}
          />
        </section>
      </div>
    </div>
  );
}
