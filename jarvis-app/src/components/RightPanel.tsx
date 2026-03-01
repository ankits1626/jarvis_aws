import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TranscriptDisplay } from './TranscriptDisplay';
// import { CoPilotPanel } from './CoPilotPanel'; // Replaced by CoPilotCardStack
import { CoPilotCardStack } from './CoPilotCardStack';
import RecordingDetailPanel from './RecordingDetailPanel';
import GemDetailPanel from './GemDetailPanel';
import ChatPanel from './ChatPanel';
import ProjectResearchChat from './ProjectResearchChat';
import { RecordingMetadata, TranscriptionSegment, RecordingTranscriptionState, CoPilotState, CoPilotStatus } from '../state/types';

type ActiveNav = 'record' | 'recordings' | 'gems' | 'projects' | 'youtube' | 'browser' | 'settings';

interface RightPanelProps {
  activeNav: ActiveNav;
  selectedRecording: string | null;
  selectedGemId: string | null;
  recordingState: 'idle' | 'recording' | 'processing';
  transcript: TranscriptionSegment[];
  transcriptionStatus: 'idle' | 'active' | 'error' | 'disabled';
  transcriptionError: string | null;
  audioUrl: string | null;
  onClosePlayer: () => void;
  recordingStates: Record<string, RecordingTranscriptionState>;
  onTranscribeRecording: (filename: string) => void;
  onSaveGem: () => void;
  onDeleteGem: () => void;
  onTranscribeGem: () => void;
  onEnrichGem: () => void;
  aiAvailable: boolean;
  recordings: RecordingMetadata[];
  currentRecording: string | null;
  copilotEnabled: boolean;
  copilotStatus: CoPilotStatus;
  copilotState: CoPilotState | null;
  copilotError: string | null;
  onDismissCopilotQuestion: (index: number) => void;
  onStartChat?: (filename: string) => void;
  chatSessionId?: string | null;
  chatStatus?: 'preparing' | 'ready' | 'error';
  selectedProjectId?: string | null;
  selectedProjectTitle?: string | null;
  onProjectGemsChanged?: () => void;
  style?: React.CSSProperties;
}

export default function RightPanel({
  activeNav,
  selectedRecording,
  selectedGemId,
  recordingState,
  transcript,
  transcriptionStatus,
  transcriptionError,
  audioUrl,
  onClosePlayer,
  recordingStates,
  onTranscribeRecording,
  onSaveGem,
  onDeleteGem,
  onTranscribeGem,
  onEnrichGem,
  aiAvailable,
  recordings,
  currentRecording,
  copilotEnabled,
  copilotStatus,
  copilotState,
  copilotError,
  onDismissCopilotQuestion,
  onStartChat,
  chatSessionId,
  chatStatus = 'ready',
  selectedProjectId,
  selectedProjectTitle,
  onProjectGemsChanged,
  style
}: RightPanelProps) {
  const [activeTab, setActiveTab] = useState<'transcript' | 'copilot' | 'chat'>('transcript');
  const [hasSeenCopilotUpdate, setHasSeenCopilotUpdate] = useState(false);

  // Knowledge file viewer state for gems
  const [openKnowledgeFiles, setOpenKnowledgeFiles] = useState<string[]>([]);
  const [activeGemTab, setActiveGemTab] = useState<'detail' | string>('detail');
  const [knowledgeFileContents, setKnowledgeFileContents] = useState<Record<string, string>>({});

  // Auto-switch to chat tab when a new session starts
  useEffect(() => {
    if (chatSessionId) {
      setActiveTab('chat');
    }
  }, [chatSessionId]);

  // Reset knowledge file viewer state when gem changes
  useEffect(() => {
    setOpenKnowledgeFiles([]);
    setActiveGemTab('detail');
    setKnowledgeFileContents({});
  }, [selectedGemId]);

  // Show notification dot when copilot has new data and user is on transcript tab
  const showNotificationDot = copilotEnabled && 
    activeTab === 'transcript' && 
    copilotState && 
    copilotState.cycle_metadata.cycle_number > 0 &&
    !hasSeenCopilotUpdate;

  // Clear notification when user switches to copilot tab
  const handleTabChange = (tab: 'transcript' | 'copilot' | 'chat') => {
    setActiveTab(tab);
    if (tab === 'copilot') {
      setHasSeenCopilotUpdate(true);
    }
  };

  // Reset notification when copilot state updates
  if (copilotState && copilotState.cycle_metadata.cycle_number > 0 && activeTab === 'transcript') {
    if (hasSeenCopilotUpdate) {
      setHasSeenCopilotUpdate(false);
    }
  }

  // Default to Research tab when entering projects view
  useEffect(() => {
    if (activeNav === 'projects' && selectedProjectId) {
      setActiveTab('chat'); // 'chat' = Research tab in projects context
    }
  }, [activeNav, selectedProjectId]);

  // Switch to Detail tab when a gem is selected in projects view
  useEffect(() => {
    if (activeNav === 'projects' && selectedGemId) {
      setActiveTab('transcript'); // 'transcript' = Detail tab in projects context
      setActiveGemTab('detail');
    }
  }, [activeNav, selectedGemId]);

  // Knowledge file handlers for gems
  const handleOpenKnowledgeFile = async (filename: string) => {
    // Add tab if not already open
    if (!openKnowledgeFiles.includes(filename)) {
      setOpenKnowledgeFiles(prev => [...prev, filename]);
    }
    setActiveGemTab(filename);
    setActiveTab('transcript'); // Ensure we're on the gem side (for projects view)

    // Fetch content if not cached
    if (!knowledgeFileContents[filename] && selectedGemId) {
      try {
        const content = await invoke<string | null>(
          'get_gem_knowledge_subfile',
          { gemId: selectedGemId, filename }
        );
        setKnowledgeFileContents(prev => ({
          ...prev,
          [filename]: content ?? 'File not found'
        }));
      } catch (e) {
        setKnowledgeFileContents(prev => ({
          ...prev,
          [filename]: `Error loading file: ${e}`
        }));
      }
    }
  };

  const handleCloseKnowledgeTab = (filename: string) => {
    setOpenKnowledgeFiles(prev => prev.filter(f => f !== filename));
    // If closing the active tab, switch to detail or last open tab
    if (activeGemTab === filename) {
      const remaining = openKnowledgeFiles.filter(f => f !== filename);
      setActiveGemTab(remaining.length > 0 ? remaining[remaining.length - 1] : 'detail');
    }
    // Clean up cached content
    setKnowledgeFileContents(prev => {
      const next = { ...prev };
      delete next[filename];
      return next;
    });
  };
  
  // Record nav: show live transcript when recording or after recording completes
  if (activeNav === 'record') {
    const isRecording = recordingState === 'recording';
    const hasTranscript = transcript.length > 0;
    const recordingCompleted = !isRecording && hasTranscript;
    
    // Keep Co-Pilot tab visible if copilot is enabled AND (recording OR has copilot data)
    const hasCopilotData = copilotState && copilotState.cycle_metadata.cycle_number > 0;
    const showCopilotTab = (copilotEnabled && isRecording) || !!hasCopilotData;

    if (isRecording || recordingCompleted) {
      // Show tabs when copilot is enabled and recording or has data
      if (showCopilotTab) {
        return (
          <div className="right-panel" style={style}>
            <div className="record-tabs-view">
              <div className="tab-buttons">
                <button
                  className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
                  onClick={() => handleTabChange('transcript')}
                >
                  Transcript
                </button>
                <button
                  className={`tab-button ${activeTab === 'copilot' ? 'active' : ''}`}
                  onClick={() => handleTabChange('copilot')}
                >
                  Co-Pilot
                  {showNotificationDot && <span className="notification-dot" />}
                </button>
              </div>
              <div className="tab-content">
                {activeTab === 'transcript' ? (
                  <TranscriptDisplay
                    transcript={transcript}
                    status={transcriptionStatus}
                    error={transcriptionError}
                    recordingFilename={currentRecording}
                  />
                ) : (
                  <CoPilotCardStack
                    state={copilotState}
                    status={copilotStatus}
                    error={copilotError}
                    recordingState={recordingState}
                    cycleInterval={60} // TODO: Wire to settings.copilot.cycle_interval when settings state is available
                    onDismissQuestion={onDismissCopilotQuestion}
                  />
                )}
              </div>
            </div>
          </div>
        );
      }

      // Show transcript only when copilot is disabled
      return (
        <div className="right-panel" style={style}>
          <TranscriptDisplay
            transcript={transcript}
            status={transcriptionStatus}
            error={transcriptionError}
            recordingFilename={currentRecording}
          />
        </div>
      );
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Start recording to see live transcript
        </div>
      </div>
    );
  }

  // Recordings nav: show recording detail panel when a recording is selected
  if (activeNav === 'recordings') {
    if (selectedRecording && audioUrl) {
      const recording = recordings.find(r => r.filename === selectedRecording);
      if (recording) {
        const recState = recordingStates[selectedRecording] || {
          transcribing: false,
          hasGem: false,
          savingGem: false,
          gemSaved: false
        };

        // Show tabs when chat session is active
        if (chatSessionId) {
          return (
            <div className="right-panel" style={style}>
              <div className="record-tabs-view">
                <div className="tab-buttons">
                  <button
                    className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
                    onClick={() => handleTabChange('transcript')}
                  >
                    Details
                  </button>
                  <button
                    className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`}
                    onClick={() => handleTabChange('chat')}
                  >
                    Chat
                  </button>
                </div>
                <div className="tab-content">
                  {activeTab === 'transcript' ? (
                    <RecordingDetailPanel
                      recording={recording}
                      audioUrl={audioUrl}
                      onClose={onClosePlayer}
                      recordingState={recState}
                      onTranscribe={() => onTranscribeRecording(selectedRecording)}
                      onSaveGem={onSaveGem}
                      onStartChat={() => onStartChat?.(selectedRecording)}
                      aiAvailable={aiAvailable}
                    />
                  ) : (
                    <ChatPanel
                      sessionId={chatSessionId}
                      recordingFilename={selectedRecording}
                      status={chatStatus}
                      preparingMessage="Generating transcript..."
                      placeholder="Ask me anything about this recording."
                    />
                  )}
                </div>
              </div>
            </div>
          );
        }

        // Show detail panel only when no chat session
        return (
          <div className="right-panel" style={style}>
            <RecordingDetailPanel
              recording={recording}
              audioUrl={audioUrl}
              onClose={onClosePlayer}
              recordingState={recState}
              onTranscribe={() => onTranscribeRecording(selectedRecording)}
              onSaveGem={onSaveGem}
              onStartChat={() => onStartChat?.(selectedRecording)}
              aiAvailable={aiAvailable}
            />
          </div>
        );
      }
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Select a recording to play or transcribe
        </div>
      </div>
    );
  }

  // Gems nav: show gem detail panel when a gem is selected
  if (activeNav === 'gems') {
    if (selectedGemId) {
      // Tabbed mode: when knowledge files are open
      if (openKnowledgeFiles.length > 0) {
        return (
          <div className="right-panel" style={style}>
            <div className="record-tabs-view">
              <div className="tab-buttons">
                <button
                  className={`tab-button ${activeGemTab === 'detail' ? 'active' : ''}`}
                  onClick={() => setActiveGemTab('detail')}
                >
                  Detail
                </button>
                {openKnowledgeFiles.map(filename => (
                  <button
                    key={filename}
                    className={`tab-button ${activeGemTab === filename ? 'active' : ''}`}
                    onClick={() => setActiveGemTab(filename)}
                  >
                    {filename}
                    <span
                      className="tab-close"
                      onClick={(e) => { e.stopPropagation(); handleCloseKnowledgeTab(filename); }}
                    >
                      ×
                    </span>
                  </button>
                ))}
              </div>
              <div className="tab-content">
                {activeGemTab === 'detail' ? (
                  <GemDetailPanel
                    gemId={selectedGemId}
                    onDelete={onDeleteGem}
                    onTranscribe={onTranscribeGem}
                    onEnrich={onEnrichGem}
                    aiAvailable={aiAvailable}
                    onOpenKnowledgeFile={handleOpenKnowledgeFile}
                  />
                ) : (
                  <div className="knowledge-file-viewer">
                    {knowledgeFileContents[activeGemTab] ? (
                      <pre className="knowledge-file-content">{knowledgeFileContents[activeGemTab]}</pre>
                    ) : (
                      <div className="loading">Loading {activeGemTab}...</div>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      }

      // Single-panel mode: no knowledge files open (current behavior)
      return (
        <div className="right-panel" style={style}>
          <GemDetailPanel
            gemId={selectedGemId}
            onDelete={onDeleteGem}
            onTranscribe={onTranscribeGem}
            onEnrich={onEnrichGem}
            aiAvailable={aiAvailable}
            onOpenKnowledgeFile={handleOpenKnowledgeFile}
          />
        </div>
      );
    }

    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Select a gem to view details
        </div>
      </div>
    );
  }

  // Projects nav: show research chat when a project is selected
  if (activeNav === 'projects') {
    // No project selected
    if (!selectedProjectId) {
      return (
        <div className="right-panel" style={style}>
          <div className="right-panel-placeholder">
            Select a project to start researching
          </div>
        </div>
      );
    }

    // Project selected + gem selected → tabs: Research | Detail | [knowledge files]
    if (selectedGemId) {
      return (
        <div className="right-panel" style={style}>
          <div className="record-tabs-view">
            <div className="tab-buttons">
              <button
                className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`}
                onClick={() => handleTabChange('chat')}
              >
                Research
              </button>
              <button
                className={`tab-button ${activeTab === 'transcript' && activeGemTab === 'detail' ? 'active' : ''}`}
                onClick={() => { handleTabChange('transcript'); setActiveGemTab('detail'); }}
              >
                Detail
              </button>
              {openKnowledgeFiles.map(filename => (
                <button
                  key={filename}
                  className={`tab-button ${activeTab === 'transcript' && activeGemTab === filename ? 'active' : ''}`}
                  onClick={() => { handleTabChange('transcript'); setActiveGemTab(filename); }}
                >
                  {filename}
                  <span
                    className="tab-close"
                    onClick={(e) => { e.stopPropagation(); handleCloseKnowledgeTab(filename); }}
                  >
                    ×
                  </span>
                </button>
              ))}
            </div>
            <div className="tab-content">
              {activeTab === 'chat' ? (
                <ProjectResearchChat
                  key={selectedProjectId}
                  projectId={selectedProjectId}
                  projectTitle={selectedProjectTitle || ''}
                  onGemsAdded={onProjectGemsChanged}
                />
              ) : activeGemTab !== 'detail' && openKnowledgeFiles.includes(activeGemTab) ? (
                <div className="knowledge-file-viewer">
                  {knowledgeFileContents[activeGemTab] ? (
                    <pre className="knowledge-file-content">{knowledgeFileContents[activeGemTab]}</pre>
                  ) : (
                    <div className="loading">Loading {activeGemTab}...</div>
                  )}
                </div>
              ) : (
                <GemDetailPanel
                  gemId={selectedGemId}
                  onDelete={onDeleteGem}
                  onTranscribe={onTranscribeGem}
                  onEnrich={onEnrichGem}
                  aiAvailable={aiAvailable}
                  onOpenKnowledgeFile={handleOpenKnowledgeFile}
                />
              )}
            </div>
          </div>
        </div>
      );
    }

    // Project selected, no gem selected → research chat full-height
    return (
      <div className="right-panel" style={style}>
        <ProjectResearchChat
          key={selectedProjectId}
          projectId={selectedProjectId}
          projectTitle={selectedProjectTitle || ''}
          onGemsAdded={onProjectGemsChanged}
        />
      </div>
    );
  }

  // For youtube, browser, settings: return null (no right panel content)
  return null;
}
