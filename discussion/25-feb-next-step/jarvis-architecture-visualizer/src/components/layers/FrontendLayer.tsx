import { useState } from 'react'
import NodeCard from '../shared/NodeCard.tsx'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

const COMPONENTS = [
  { name: 'App', icon: '📱', description: 'Root component — layout, state reducer, conditional rendering', commands: ['get_recordings', 'get_settings'], events: ['recording-started', 'recording-stopped'], file: 'src/App.tsx' },
  { name: 'RecordButton', icon: '⏺️', description: 'Start/stop recording — main user interaction entry point', commands: ['start_recording', 'stop_recording'], events: [], file: 'src/components/RecordButton.tsx' },
  { name: 'RecordingsList', icon: '📋', description: 'Lists all recordings with timestamps and durations', commands: ['get_recordings'], events: [], file: 'src/components/RecordingsList.tsx' },
  { name: 'RecordingRow', icon: '📄', description: 'Single recording entry — play, delete, view transcript', commands: ['delete_recording'], events: [], file: 'src/components/RecordingRow.tsx' },
  { name: 'AudioPlayer', icon: '🔊', description: 'HTML5 audio playback controls', commands: [], events: [], file: 'src/components/AudioPlayer.tsx' },
  { name: 'TranscriptDisplay', icon: '📝', description: 'Real-time transcription — partial (gray) + final (white)', commands: [], events: ['transcription-update'], file: 'src/components/TranscriptDisplay.tsx' },
  { name: 'StatusIndicator', icon: '🔵', description: 'Recording status dot — idle/processing/recording', commands: [], events: [], file: 'src/components/StatusIndicator.tsx' },
  { name: 'Settings', icon: '⚙️', description: 'VAD, Vosk, Whisper toggles — engine configuration', commands: ['get_settings', 'update_settings'], events: [], file: 'src/components/Settings.tsx' },
  { name: 'ModelList', icon: '📦', description: 'Download and manage Whisper models', commands: ['get_available_models', 'download_model', 'delete_model'], events: ['model-download-progress'], file: 'src/components/ModelList.tsx' },
  { name: 'BrowserTool', icon: '🌐', description: 'Browser tab listing — capture content from open tabs', commands: ['get_browser_tabs', 'capture_tab'], events: [], file: 'src/components/BrowserTool.tsx' },
  { name: 'YouTubeSection', icon: '▶️', description: 'YouTube video detection badge and metadata display', commands: ['fetch_youtube_gist'], events: ['youtube-video-detected'], file: 'src/components/YouTubeSection.tsx' },
  { name: 'GemsPanel', icon: '💎', description: 'Knowledge base browser — search, filter, manage gems', commands: ['list_gems', 'search_gems', 'delete_gem'], events: [], file: 'src/components/GemsPanel.tsx' },
  { name: 'PermissionDialog', icon: '🔐', description: 'macOS permission prompts for screen recording / accessibility', commands: [], events: [], file: 'src/components/PermissionDialog.tsx' },
  { name: 'DeleteConfirmDialog', icon: '🗑️', description: 'Confirmation modal before destructive actions', commands: [], events: [], file: 'src/components/DeleteConfirmDialog.tsx' },
  { name: 'ErrorToast', icon: '⚠️', description: 'Error notification — auto-dismiss after timeout', commands: [], events: [], file: 'src/components/ErrorToast.tsx' },
]

type StateName = 'idle' | 'processing' | 'recording'

const STATE_TRANSITIONS: { from: StateName; to: StateName; action: string }[] = [
  { from: 'idle', to: 'processing', action: 'START_RECORDING' },
  { from: 'processing', to: 'recording', action: 'RECORDING_STARTED' },
  { from: 'recording', to: 'processing', action: 'STOP_RECORDING' },
  { from: 'processing', to: 'idle', action: 'RECORDING_STOPPED' },
]

const HOOKS = [
  { name: 'useRecording', description: 'Manages recording lifecycle — start, stop, status, current recording', params: 'none', returns: '{ isRecording, start, stop, currentRecording }', usedBy: ['App', 'RecordButton'], file: 'src/hooks/useRecording.ts' },
  { name: 'useTauriCommand', description: 'Generic wrapper around Tauri invoke() — handles loading, error, data', params: '<T>(command: string, args?: object)', returns: '{ data, loading, error, execute }', usedBy: ['BrowserTool', 'GemsPanel', 'Settings'], file: 'src/hooks/useTauriCommand.ts' },
  { name: 'useTauriEvent', description: 'Subscribes to Tauri events — auto-cleanup on unmount', params: '<T>(event: string, handler: (payload: T) => void)', returns: 'void', usedBy: ['TranscriptDisplay', 'YouTubeSection', 'ModelList'], file: 'src/hooks/useTauriEvent.ts' },
]

// ── Component ──

export default function FrontendLayer() {
  const [activeState, setActiveState] = useState<StateName>('idle')

  const stateColors: Record<StateName, string> = {
    idle: 'border-slate-500 bg-slate-500/10 text-slate-300',
    processing: 'border-cyan-500 bg-cyan-500/10 text-cyan-400',
    recording: 'border-red-500 bg-red-500/10 text-red-400',
  }

  function handleTransition(to: StateName) {
    setActiveState(to)
  }

  return (
    <div className="space-y-10">
      {/* ── Component Tree ── */}
      <section>
        <SectionHeader
          title="Component Tree"
          description="14 React components — click any to see its Tauri commands and event subscriptions"
        />
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
          {COMPONENTS.map((c) => (
            <NodeCard
              key={c.name}
              title={c.name}
              icon={c.icon}
              description={c.description}
              color="border-cyan-500/20"
            >
              <div className="space-y-3 text-xs">
                <p className="text-slate-400">{c.description}</p>
                {c.commands.length > 0 && (
                  <div>
                    <span className="text-cyan-500 font-medium">Commands:</span>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {c.commands.map((cmd) => (
                        <span key={cmd} className="font-mono px-1.5 py-0.5 rounded bg-cyan-500/10 text-cyan-300 text-[11px]">
                          {cmd}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
                {c.events.length > 0 && (
                  <div>
                    <span className="text-amber-500 font-medium">Listens to:</span>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {c.events.map((ev) => (
                        <span key={ev} className="font-mono px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-300 text-[11px]">
                          {ev}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
                <p className="font-mono text-slate-600">{c.file}</p>
              </div>
            </NodeCard>
          ))}
        </div>
      </section>

      {/* ── State Machine ── */}
      <section>
        <SectionHeader
          title="State Machine"
          description="Click a state to simulate transitions — the app uses useReducer with atomic state changes"
        />
        <div className="flex items-center justify-center gap-4 flex-wrap py-6">
          {(['idle', 'processing', 'recording'] as const).map((state) => {
            const isActive = activeState === state
            const transitions = STATE_TRANSITIONS.filter(t => t.from === state)
            return (
              <div key={state} className="flex items-center gap-4">
                <button
                  onClick={() => handleTransition(state)}
                  className={`px-5 py-3 rounded-xl border-2 font-mono text-sm font-medium transition-all ${
                    isActive
                      ? `${stateColors[state]} scale-110 shadow-lg`
                      : 'border-slate-700 bg-slate-900/50 text-slate-500 hover:border-slate-600'
                  }`}
                >
                  {state}
                </button>
                {transitions.length > 0 && state !== 'recording' && (
                  <div className="flex items-center gap-1 text-slate-600">
                    <span className="text-xs">&rarr;</span>
                    <span className="text-[10px] font-mono text-slate-500">
                      {transitions[0].action}
                    </span>
                    <span className="text-xs">&rarr;</span>
                  </div>
                )}
              </div>
            )
          })}
        </div>
        <div className="text-center mt-2">
          <span className="text-xs text-slate-600">
            Current: <span className={`font-mono font-medium ${activeState === 'idle' ? 'text-slate-300' : activeState === 'processing' ? 'text-cyan-400' : 'text-red-400'}`}>{activeState}</span>
          </span>
        </div>
        <div className="mt-4">
          <CodeSnippet
            language="typescript"
            code={`// state/reducer.ts
type AppState = {
  status: 'idle' | 'processing' | 'recording'
  recordings: Recording[]
  error: string | null
}

type Action =
  | { type: 'START_RECORDING' }
  | { type: 'RECORDING_STARTED'; recording: Recording }
  | { type: 'STOP_RECORDING' }
  | { type: 'RECORDING_STOPPED' }
  | { type: 'SET_ERROR'; error: string }`}
          />
        </div>
      </section>

      {/* ── Hook System ── */}
      <section>
        <SectionHeader
          title="Custom Hooks"
          description="Three hooks bridge React components to Tauri's backend"
        />
        <div className="space-y-2">
          {HOOKS.map((h) => (
            <NodeCard
              key={h.name}
              title={h.name}
              icon="🪝"
              description={h.description}
              color="border-cyan-500/20"
              badge={h.file.split('/').pop()}
              badgeColor="text-cyan-500/60"
            >
              <div className="space-y-2 text-xs">
                <p className="text-slate-400">{h.description}</p>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <span className="text-slate-500 font-medium">Params:</span>
                    <p className="font-mono text-cyan-300/80 mt-0.5">{h.params}</p>
                  </div>
                  <div>
                    <span className="text-slate-500 font-medium">Returns:</span>
                    <p className="font-mono text-cyan-300/80 mt-0.5">{h.returns}</p>
                  </div>
                </div>
                <div>
                  <span className="text-slate-500 font-medium">Used by:</span>
                  <div className="flex gap-1 mt-1">
                    {h.usedBy.map((c) => (
                      <span key={c} className="px-1.5 py-0.5 rounded bg-slate-800 text-slate-300 text-[11px]">{c}</span>
                    ))}
                  </div>
                </div>
              </div>
            </NodeCard>
          ))}
        </div>
      </section>

      <KeyFiles
        color="text-cyan-400"
        files={[
          { path: 'src/App.tsx', description: 'Root component — state reducer, conditional rendering, layout' },
          { path: 'src/state/reducer.ts', description: 'State machine — atomic transitions between idle/processing/recording' },
          { path: 'src/state/types.ts', description: 'TypeScript types mirroring Rust structs' },
          { path: 'src/hooks/useRecording.ts', description: 'Recording lifecycle hook — start, stop, status' },
          { path: 'src/hooks/useTauriCommand.ts', description: 'Generic Tauri invoke() wrapper' },
          { path: 'src/hooks/useTauriEvent.ts', description: 'Tauri event listener with auto-cleanup' },
          { path: 'src/App.css', description: '35KB monolithic CSS — component-specific sections' },
        ]}
      />
    </div>
  )
}
