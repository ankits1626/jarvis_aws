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
  WhisperKitStatus,
} from '../state/types';

interface SettingsProps {
  onClose: () => void;
}

export function Settings({ onClose }: SettingsProps) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [whisperKitModels, setWhisperKitModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [whisperKitStatus, setWhisperKitStatus] = useState<WhisperKitStatus | null>(null);

  // Load settings and models on mount
  useEffect(() => {
    const loadData = async () => {
      try {
        setLoading(true);
        const [settingsData, modelsData, whisperKitModelsData, whisperKitStatusData] = await Promise.all([
          invoke<Settings>('get_settings'),
          invoke<ModelInfo[]>('list_models'),
          invoke<ModelInfo[]>('list_whisperkit_models'),
          invoke<WhisperKitStatus>('check_whisperkit_status'),
        ]);
        setSettings(settingsData);
        setModels(modelsData);
        setWhisperKitModels(whisperKitModelsData);
        setWhisperKitStatus(whisperKitStatusData);
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

  // Listen for model-download-progress events (updates both whisper and whisperkit models)
  useEffect(() => {
    const unlisten = listen<ModelProgressEvent>('model-download-progress', (event) => {
      const updateModels = (prevModels: ModelInfo[]) =>
        prevModels.map((model) =>
          model.filename === event.payload.model_name
            ? {
                ...model,
                status: { type: 'downloading' as const, progress: event.payload.progress },
              }
            : model
        );
      setModels(updateModels);
      setWhisperKitModels(updateModels);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for model-download-complete events (refreshes both model lists)
  useEffect(() => {
    const unlisten = listen<ModelDownloadCompleteEvent>('model-download-complete', async () => {
      // Refresh both model lists to get updated status
      try {
        const [modelsData, whisperKitModelsData] = await Promise.all([
          invoke<ModelInfo[]>('list_models'),
          invoke<ModelInfo[]>('list_whisperkit_models'),
        ]);
        setModels(modelsData);
        setWhisperKitModels(whisperKitModelsData);
      } catch (err) {
        console.error('Failed to refresh models:', err);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for model-download-error events (updates both whisper and whisperkit models)
  useEffect(() => {
    const unlisten = listen<ModelDownloadErrorEvent>('model-download-error', (event) => {
      const updateModels = (prevModels: ModelInfo[]) =>
        prevModels.map((model) =>
          model.filename === event.payload.model_name
            ? {
                ...model,
                status: { type: 'error' as const, message: event.payload.error },
              }
            : model
        );
      setModels(updateModels);
      setWhisperKitModels(updateModels);
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

  const handleWindowDurationChange = async (duration: number) => {
    try {
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          window_duration: duration,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleEngineChange = async (engine: "whisper-rs" | "whisperkit") => {
    try {
      const updatedSettings = {
        ...settings,
        transcription: {
          ...settings.transcription,
          transcription_engine: engine,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCheckWhisperKitAgain = async () => {
    try {
      const status = await invoke<WhisperKitStatus>('check_whisperkit_status');
      setWhisperKitStatus(status);
      
      // If now available, also refresh the model list
      if (status.available) {
        const modelsData = await invoke<ModelInfo[]>('list_whisperkit_models');
        setWhisperKitModels(modelsData);
      }
    } catch (err) {
      console.error('Failed to check WhisperKit status:', err);
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
          <h3>Transcription Engine</h3>
          <div className="engine-options">
            <label className="engine-option">
              <input
                type="radio"
                name="engine"
                value="whisper-rs"
                checked={settings.transcription.transcription_engine === "whisper-rs"}
                onChange={() => handleEngineChange("whisper-rs")}
              />
              <span>whisper.cpp (Metal GPU)</span>
            </label>
            <label className={`engine-option ${!whisperKitStatus?.available ? 'engine-unavailable' : ''}`}>
              <input
                type="radio"
                name="engine"
                value="whisperkit"
                checked={settings.transcription.transcription_engine === "whisperkit"}
                disabled={!whisperKitStatus?.available}
                onChange={() => handleEngineChange("whisperkit")}
              />
              <span>WhisperKit (Apple Neural Engine)</span>
              {!whisperKitStatus?.available && whisperKitStatus?.reason && (
                <span className="engine-reason"> — {whisperKitStatus.reason}</span>
              )}
            </label>
          </div>
          {!whisperKitStatus?.available && (
            <div className="whisperkit-install-info">
              <p>WhisperKit requires Apple Silicon and macOS 14+.</p>
              <p>Install: <code>brew install whisperkit-cli</code></p>
              <button onClick={handleCheckWhisperKitAgain} className="check-again-button">
                Check Again
              </button>
            </div>
          )}
          <p className="engine-note">Engine changes take effect after app restart.</p>
        </section>

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
          <h3>Audio Window</h3>
          <div className="setting-row">
            <label htmlFor="window-duration">
              Window Duration: {settings.transcription.window_duration.toFixed(1)}s
            </label>
            <input
              type="range"
              id="window-duration"
              min="1"
              max="10"
              step="0.5"
              value={settings.transcription.window_duration}
              onChange={(e) => handleWindowDurationChange(parseFloat(e.target.value))}
            />
            <p className="setting-info">
              Shorter = lower latency, longer = better accuracy. Takes effect on next recording.
            </p>
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
          <h3>
            {settings.transcription.transcription_engine === "whisperkit" 
              ? "WhisperKit Models" 
              : "Whisper Models"}
          </h3>
          {settings.transcription.transcription_engine === "whisperkit" ? (
            <ModelList
              models={whisperKitModels}
              selectedModel={settings.transcription.whisperkit_model}
              onModelSelected={async () => {
                // Refresh model list after selection or deletion
                try {
                  const modelsData = await invoke<ModelInfo[]>('list_whisperkit_models');
                  setWhisperKitModels(modelsData);
                } catch (err) {
                  console.error('Failed to refresh WhisperKit models:', err);
                }
              }}
              onDownloadStarted={(modelName) => {
                // Optimistic UI update: show downloading state immediately
                setWhisperKitModels((prev) =>
                  prev.map((m) =>
                    m.filename === modelName
                      ? { ...m, status: { type: 'downloading', progress: 0 } }
                      : m
                  )
                );
              }}
              downloadCommand="download_whisperkit_model"
              cancelCommand={undefined}
              deleteCommand={undefined}
              settingsField="whisperkit_model"
            />
          ) : (
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
              downloadCommand="download_model"
              cancelCommand="cancel_download"
              deleteCommand="delete_model"
              settingsField="whisper_model"
            />
          )}
        </section>
      </div>
    </div>
  );
}
