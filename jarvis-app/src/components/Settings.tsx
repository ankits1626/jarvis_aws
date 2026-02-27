import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ModelList } from './ModelList';
import type {
  Settings,
  ModelInfo,
  LlmModelInfo,
  ModelProgressEvent,
  LlmModelProgressEvent,
  ModelDownloadCompleteEvent,
  LlmModelDownloadCompleteEvent,
  ModelDownloadErrorEvent,
  LlmModelDownloadErrorEvent,
  SettingsChangedEvent,
  WhisperKitStatus,
  MlxDiagnostics,
  MlxVenvProgressEvent,
} from '../state/types';

interface BrowserSettings {
  observer_enabled: boolean;
}

interface SettingsProps {
  onClose: () => void;
}

export function Settings({ onClose }: SettingsProps) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [browserSettings, setBrowserSettings] = useState<BrowserSettings | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [whisperKitModels, setWhisperKitModels] = useState<ModelInfo[]>([]);
  const [llmModels, setLlmModels] = useState<LlmModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [whisperKitStatus, setWhisperKitStatus] = useState<WhisperKitStatus | null>(null);
  const [mlxDiagnostics, setMlxDiagnostics] = useState<MlxDiagnostics | null>(null);
  const [venvSetupInProgress, setVenvSetupInProgress] = useState(false);
  const [venvSetupPhase, setVenvSetupPhase] = useState<string | null>(null);
  const [venvSetupError, setVenvSetupError] = useState<string | null>(null);

  // Load settings and models on mount
  useEffect(() => {
    const loadData = async () => {
      try {
        setLoading(true);
        const [settingsData, browserSettingsData, modelsData, whisperKitModelsData, whisperKitStatusData, llmModelsData] = await Promise.all([
          invoke<Settings>('get_settings'),
          invoke<BrowserSettings>('get_browser_settings'),
          invoke<ModelInfo[]>('list_models'),
          invoke<ModelInfo[]>('list_whisperkit_models'),
          invoke<WhisperKitStatus>('check_whisperkit_status'),
          invoke<LlmModelInfo[]>('list_llm_models'),
        ]);
        setSettings(settingsData);
        setBrowserSettings(browserSettingsData);
        setModels(modelsData);
        setWhisperKitModels(whisperKitModelsData);
        setWhisperKitStatus(whisperKitStatusData);
        setLlmModels(llmModelsData);
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

  // Check MLX diagnostics when provider is MLX
  useEffect(() => {
    if (settings?.intelligence.provider === 'mlx') {
      const checkDiagnostics = async () => {
        try {
          const diagnostics = await invoke<MlxDiagnostics>('check_mlx_dependencies');
          setMlxDiagnostics(diagnostics);
        } catch (err) {
          console.error('Failed to check MLX diagnostics:', err);
          setMlxDiagnostics(null);
        }
      };
      checkDiagnostics();
    } else {
      setMlxDiagnostics(null);
    }
  }, [settings?.intelligence.provider]);

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

  // Listen for llm-model-download-progress events
  useEffect(() => {
    const unlisten = listen<LlmModelProgressEvent>('llm-model-download-progress', (event) => {
      setLlmModels((prevModels) =>
        prevModels.map((model) =>
          model.id === event.payload.model_id
            ? {
                ...model,
                status: { type: 'downloading' as const, progress: event.payload.progress },
              }
            : model
        )
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for llm-model-download-complete events
  useEffect(() => {
    const unlisten = listen<LlmModelDownloadCompleteEvent>('llm-model-download-complete', async () => {
      // Refresh LLM model list to get updated status
      try {
        const llmModelsData = await invoke<LlmModelInfo[]>('list_llm_models');
        setLlmModels(llmModelsData);
      } catch (err) {
        console.error('Failed to refresh LLM models:', err);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for MLX venv setup events
  useEffect(() => {
    const unlistenProgress = listen<MlxVenvProgressEvent>('mlx-venv-setup-progress', (event) => {
      setVenvSetupInProgress(true);
      setVenvSetupPhase(event.payload.message);
      setVenvSetupError(null);
    });

    const unlistenComplete = listen('mlx-venv-setup-complete', async () => {
      setVenvSetupInProgress(false);
      setVenvSetupPhase(null);
      // Refresh diagnostics after setup completes
      try {
        const diagnostics = await invoke<MlxDiagnostics>('check_mlx_dependencies');
        setMlxDiagnostics(diagnostics);
      } catch (err) {
        console.error('Failed to refresh diagnostics:', err);
      }
    });

    const unlistenError = listen<{ error: string }>('mlx-venv-setup-error', (event) => {
      setVenvSetupInProgress(false);
      setVenvSetupPhase(null);
      setVenvSetupError(event.payload.error);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Listen for llm-model-download-error events
  useEffect(() => {
    const unlisten = listen<LlmModelDownloadErrorEvent>('llm-model-download-error', (event) => {
      setLlmModels((prevModels) =>
        prevModels.map((model) =>
          model.id === event.payload.model_id
            ? {
                ...model,
                status: { type: 'error' as const, message: event.payload.error },
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

  const handleEngineChange = async (engine: "whisper-rs" | "whisperkit" | "mlx-omni") => {
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

  const handleBrowserObserverChange = async (enabled: boolean) => {
    try {
      await invoke('update_browser_settings', { observerEnabled: enabled });
      setBrowserSettings({ observer_enabled: enabled });
    } catch (err) {
      console.error('Failed to update browser settings:', err);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleProviderChange = async (provider: string) => {
    try {
      const updatedSettings = {
        ...settings!,
        intelligence: {
          ...settings!.intelligence,
          provider,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleLlmModelSwitch = async (modelId: string) => {
    try {
      await invoke('switch_llm_model', { modelId });
      // Refresh settings (active_model changed) and model list
      const [updatedSettings, llmModelsData] = await Promise.all([
        invoke<Settings>('get_settings'),
        invoke<LlmModelInfo[]>('list_llm_models'),
      ]);
      setSettings(updatedSettings);
      setLlmModels(llmModelsData);
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
            <label className="engine-option">
              <input
                type="radio"
                name="engine"
                value="mlx-omni"
                checked={settings.transcription.transcription_engine === "mlx-omni"}
                onChange={() => handleEngineChange("mlx-omni")}
              />
              <span>MLX Omni (Local, Private)</span>
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
          <p className="engine-note">
            {settings.transcription.transcription_engine === "mlx-omni" 
              ? "Real-time transcription during recording still uses Whisper for instant feedback. MLX Omni provides accurate multilingual transcripts after recording completes."
              : "Engine changes take effect after app restart."}
          </p>

          {settings.transcription.transcription_engine === "mlx-omni" && (
            <div className="multimodal-models-panel" style={{
              marginTop: '16px',
              padding: '16px',
              backgroundColor: '#f8f9fa',
              border: '1px solid #dee2e6',
              borderRadius: '4px'
            }}>
              <h4 style={{ marginTop: 0, marginBottom: '12px' }}>Multimodal Models</h4>
              
              {/* Venv status indicator */}
              {mlxDiagnostics && mlxDiagnostics.venv_status === 'ready' && (
                <div style={{
                  padding: '8px 12px',
                  marginBottom: '12px',
                  backgroundColor: '#d4edda',
                  border: '1px solid #c3e6cb',
                  borderRadius: '4px',
                  color: '#155724',
                  fontSize: '13px'
                }}>
                  ✓ Venv Ready
                </div>
              )}
              
              {mlxDiagnostics && mlxDiagnostics.venv_status !== 'ready' && (
                <div style={{
                  padding: '8px 12px',
                  marginBottom: '12px',
                  backgroundColor: '#fff3cd',
                  border: '1px solid #ffc107',
                  borderRadius: '4px',
                  color: '#856404',
                  fontSize: '13px'
                }}>
                  ⚠ Venv needs setup (see MLX Models section below)
                </div>
              )}

              {/* Filter multimodal models (those with "audio" capability) */}
              {llmModels.filter(m => m.capabilities?.includes('audio')).length === 0 ? (
                <p style={{ margin: 0, color: '#6c757d', fontSize: '14px' }}>
                  Download a multimodal model to enable MLX transcription
                </p>
              ) : (
                <div className="multimodal-model-list">
                  {llmModels
                    .filter(m => m.capabilities?.includes('audio'))
                    .map(model => (
                      <div
                        key={model.id}
                        className="multimodal-model-card"
                        style={{
                          padding: '12px',
                          marginBottom: '8px',
                          backgroundColor: 'white',
                          border: model.id === settings.transcription.mlx_omni_model ? '2px solid #007bff' : '1px solid #dee2e6',
                          borderRadius: '4px',
                          cursor: model.status.type === 'downloaded' ? 'pointer' : 'default'
                        }}
                        onClick={() => {
                          if (model.status.type === 'downloaded' && model.id !== settings.transcription.mlx_omni_model) {
                            // Update mlx_omni_model setting
                            const updatedSettings = {
                              ...settings,
                              transcription: {
                                ...settings.transcription,
                                mlx_omni_model: model.id,
                              },
                            };
                            invoke('update_settings', { settings: updatedSettings }).catch(err => {
                              setError(err instanceof Error ? err.message : String(err));
                            });
                          }
                        }}
                      >
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                          <div style={{ flex: 1 }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                              <input
                                type="radio"
                                name="mlx-omni-model"
                                checked={model.id === settings.transcription.mlx_omni_model}
                                disabled={model.status.type !== 'downloaded'}
                                onChange={() => {}}
                                style={{ margin: 0 }}
                              />
                              <strong style={{ fontSize: '14px' }}>{model.display_name}</strong>
                              {model.id === settings.transcription.mlx_omni_model && model.status.type === 'downloaded' && (
                                <span style={{
                                  padding: '2px 8px',
                                  backgroundColor: '#007bff',
                                  color: 'white',
                                  borderRadius: '3px',
                                  fontSize: '11px',
                                  fontWeight: 'bold'
                                }}>
                                  ACTIVE
                                </span>
                              )}
                            </div>
                            <div style={{ fontSize: '12px', color: '#6c757d', marginTop: '4px', marginLeft: '24px' }}>
                              {model.size_estimate} • {model.quality_tier} quality
                            </div>
                            <div style={{ fontSize: '12px', color: '#6c757d', marginLeft: '24px' }}>
                              {model.description}
                            </div>
                          </div>
                          <div>
                            {model.status.type === 'not_downloaded' && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  invoke('download_llm_model', { modelId: model.id }).catch(err => {
                                    setError(err instanceof Error ? err.message : String(err));
                                  });
                                  setLlmModels(prev =>
                                    prev.map(m =>
                                      m.id === model.id
                                        ? { ...m, status: { type: 'downloading', progress: 0 } }
                                        : m
                                    )
                                  );
                                }}
                                style={{
                                  padding: '6px 12px',
                                  backgroundColor: '#007bff',
                                  color: 'white',
                                  border: 'none',
                                  borderRadius: '4px',
                                  cursor: 'pointer',
                                  fontSize: '12px'
                                }}
                              >
                                Download
                              </button>
                            )}
                            {model.status.type === 'downloading' && (
                              <div style={{ textAlign: 'right' }}>
                                <div style={{ fontSize: '12px', color: '#6c757d', marginBottom: '4px' }}>
                                  {Math.round(model.status.progress)}%
                                </div>
                                <div style={{
                                  width: '100px',
                                  height: '4px',
                                  backgroundColor: '#e9ecef',
                                  borderRadius: '2px',
                                  overflow: 'hidden'
                                }}>
                                  <div style={{
                                    width: `${model.status.progress}%`,
                                    height: '100%',
                                    backgroundColor: '#007bff',
                                    transition: 'width 0.3s ease'
                                  }} />
                                </div>
                              </div>
                            )}
                            {model.status.type === 'downloaded' && model.id !== settings.transcription.mlx_omni_model && (
                              <span style={{ fontSize: '12px', color: '#28a745' }}>✓ Downloaded</span>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                </div>
              )}
            </div>
          )}
        </section>

        <section className="settings-section">
          <h3>Browser</h3>
          <div className="setting-row">
            <label htmlFor="browser-observer">
              <input
                type="checkbox"
                id="browser-observer"
                checked={browserSettings?.observer_enabled ?? true}
                onChange={(e) => handleBrowserObserverChange(e.target.checked)}
              />
              Automatically detect YouTube videos in Chrome
            </label>
            <p className="setting-info">
              When enabled, JarvisApp will monitor Chrome and offer to prepare gists for YouTube videos you watch
            </p>
          </div>
        </section>

        <section className="settings-section">
          <h3>Intelligence Provider</h3>
          <div className="provider-options">
            <label className="provider-option">
              <input
                type="radio"
                name="provider"
                value="mlx"
                checked={settings.intelligence.provider === "mlx"}
                onChange={() => handleProviderChange("mlx")}
              />
              <span>MLX (Local, Private)</span>
            </label>
            <label className="provider-option">
              <input
                type="radio"
                name="provider"
                value="intelligencekit"
                checked={settings.intelligence.provider === "intelligencekit"}
                onChange={() => handleProviderChange("intelligencekit")}
              />
              <span>IntelligenceKit (Local, Fast)</span>
            </label>
            <label className="provider-option">
              <input
                type="radio"
                name="provider"
                value="api"
                checked={settings.intelligence.provider === "api"}
                disabled={true}
                onChange={() => handleProviderChange("api")}
              />
              <span>Cloud API (Coming Soon)</span>
            </label>
          </div>
          <p className="provider-note">Provider changes take effect immediately.</p>
        </section>

        {settings.intelligence.provider === "mlx" && (
          <section className="settings-section">
            <h3>MLX Models</h3>

            {/* Python not found */}
            {mlxDiagnostics && !mlxDiagnostics.python_found && (
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#f8d7da',
                border: '1px solid #f5c6cb',
                borderRadius: '4px',
                color: '#721c24'
              }}>
                <strong>Python not found:</strong> {mlxDiagnostics.python_error}
                <br />
                <small>Install Python 3.10+ or update the python_path setting below.</small>
              </div>
            )}

            {/* Venv setup needed */}
            {mlxDiagnostics && mlxDiagnostics.python_found && mlxDiagnostics.venv_status !== 'ready' && !venvSetupInProgress && (
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#d1ecf1',
                border: '1px solid #bee5eb',
                borderRadius: '4px',
                color: '#0c5460'
              }}>
                <strong>Python found:</strong> {mlxDiagnostics.python_version}
                <br />
                {mlxDiagnostics.venv_status === 'needs_update'
                  ? <span>MLX environment needs updating (dependencies changed).</span>
                  : <span>MLX environment not set up yet. Click below to auto-install dependencies.</span>
                }
                <div style={{ marginTop: '8px' }}>
                  <button
                    onClick={async () => {
                      setVenvSetupInProgress(true);
                      setVenvSetupPhase('Starting setup...');
                      setVenvSetupError(null);
                      try {
                        await invoke('setup_mlx_venv');
                      } catch (err) {
                        setVenvSetupInProgress(false);
                        setVenvSetupError(err instanceof Error ? err.message : String(err));
                      }
                    }}
                    style={{
                      padding: '6px 16px',
                      backgroundColor: '#0c5460',
                      color: 'white',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer',
                      fontSize: '13px',
                    }}
                  >
                    {mlxDiagnostics.venv_status === 'needs_update' ? 'Update MLX Environment' : 'Setup MLX Environment'}
                  </button>
                </div>
              </div>
            )}

            {/* Venv setup in progress */}
            {venvSetupInProgress && (
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#fff3cd',
                border: '1px solid #ffc107',
                borderRadius: '4px',
                color: '#856404'
              }}>
                <strong>Setting up MLX environment...</strong>
                <br />
                {venvSetupPhase && <span>{venvSetupPhase}</span>}
              </div>
            )}

            {/* Venv setup error */}
            {venvSetupError && (
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#f8d7da',
                border: '1px solid #f5c6cb',
                borderRadius: '4px',
                color: '#721c24'
              }}>
                <strong>Setup failed:</strong> {venvSetupError}
              </div>
            )}

            {/* Venv ready */}
            {mlxDiagnostics && mlxDiagnostics.venv_status === 'ready' && !venvSetupInProgress && (
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#d4edda',
                border: '1px solid #c3e6cb',
                borderRadius: '4px',
                color: '#155724'
              }}>
                <strong>MLX environment ready</strong>
                {mlxDiagnostics.venv_python_path && (
                  <small style={{ display: 'block', marginTop: '4px', opacity: 0.8 }}>
                    Using: {mlxDiagnostics.venv_python_path}
                  </small>
                )}
                {llmModels.filter(m => m.capabilities?.includes('text')).every(m => m.status.type === 'not_downloaded') && (
                  <span style={{ display: 'block', marginTop: '4px' }}>
                    <strong>No models downloaded yet.</strong> Download a model below to enable AI enrichment.
                  </span>
                )}
              </div>
            )}

            <ModelList
              models={llmModels
                .filter(m => m.capabilities?.includes('text'))
                .map(m => ({
                  filename: m.id,
                  display_name: m.display_name,
                  description: m.description,
                  size_estimate: m.size_estimate,
                  quality_tier: m.quality_tier,
                  status: m.status,
                }))}
              selectedModel={settings.intelligence.active_model}
              onModelSelected={async () => {
                try {
                  const llmModelsData = await invoke<LlmModelInfo[]>('list_llm_models');
                  setLlmModels(llmModelsData);
                } catch (err) {
                  console.error('Failed to refresh LLM models:', err);
                }
              }}
              onDownloadStarted={(modelId) => {
                setLlmModels((prev) =>
                  prev.map((m) =>
                    m.id === modelId
                      ? { ...m, status: { type: 'downloading', progress: 0 } }
                      : m
                  )
                );
              }}
              downloadCommand="download_llm_model"
              cancelCommand="cancel_llm_download"
              deleteCommand="delete_llm_model"
              settingsField={undefined}
              customSelectHandler={handleLlmModelSwitch}
              invokeParamKey="modelId"
              disableActiveModelDeletion={true}
            />
          </section>
        )}

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
