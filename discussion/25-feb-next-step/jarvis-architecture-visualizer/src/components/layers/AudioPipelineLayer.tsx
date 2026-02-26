import { useState } from 'react'
import NodeCard from '../shared/NodeCard.tsx'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

const PIPELINE_STAGES = [
  {
    name: 'JarvisListen',
    icon: '🎧',
    tech: 'Swift · ScreenCaptureKit',
    description: 'macOS sidecar that captures system audio + microphone. Outputs raw PCM to a file via --output flag.',
    details: 'Runs as an external binary spawned by Tauri. Captures at 16kHz, 16-bit signed integer, mono. Writes directly to ~/.jarvis/recordings/',
    color: 'border-emerald-500/30',
  },
  {
    name: 'PCM File',
    icon: '📄',
    tech: '16kHz · 16-bit · Mono',
    description: 'Raw audio stored at ~/.jarvis/recordings/<timestamp>.pcm — growing file during recording.',
    details: 'File grows in real-time as JarvisListen appends audio data. RecordingManager tracks the file path.',
    color: 'border-emerald-500/20',
  },
  {
    name: 'FIFO Pipe',
    icon: '🔗',
    tech: 'Named pipe (mkfifo)',
    description: 'Named pipe bridges the PCM file to the transcription pipeline — AudioRouter tails the PCM and writes to FIFO.',
    details: 'Created by RecordingManager, read by TranscriptionManager. Solves the producer-consumer coordination problem.',
    color: 'border-emerald-500/20',
  },
  {
    name: 'AudioRouter',
    icon: '🔀',
    tech: 'Rust · mpsc channel',
    description: 'Tails the PCM file, chunks audio into windows, sends via mpsc channel to TranscriptionManager.',
    details: 'Reads in 3-second windows with 0.5s overlap. Runs in a background tokio task.',
    color: 'border-emerald-500/20',
  },
  {
    name: 'TranscriptionManager',
    icon: '📝',
    tech: 'Rust · tokio::sync::Mutex',
    description: 'Orchestrates the hybrid transcription pipeline — distributes audio to VAD, Vosk, and Whisper.',
    details: 'Manages provider lifecycle, emits transcription-update events to frontend in real-time.',
    color: 'border-emerald-500/30',
  },
]

const TRANSCRIPTION_PROVIDERS = [
  {
    name: 'Silero VAD',
    icon: '👂',
    purpose: 'Voice Activity Detection',
    timing: '~instant',
    description: 'Detects whether audio contains speech. Gates the transcription pipeline — no speech = no processing.',
    tech: 'ONNX Runtime (ort crate)',
    color: 'text-emerald-400',
  },
  {
    name: 'Vosk',
    icon: '💨',
    purpose: 'Fast partial transcription',
    timing: '~instant',
    description: 'Produces immediate, rough transcription (shown as gray text in UI). Good enough for real-time feedback.',
    tech: 'libvosk.dylib (bundled as macOS framework)',
    color: 'text-slate-400',
  },
  {
    name: 'Whisper',
    icon: '🎯',
    purpose: 'High-quality final transcription',
    timing: '~2-5s',
    description: 'Produces accurate, final transcription (replaces gray Vosk text). Runs after VAD confirms speech.',
    tech: 'whisper.cpp (whisper-rs crate)',
    color: 'text-white',
  },
]

// ── Component ──

export default function AudioPipelineLayer() {
  const [activeProvider, setActiveProvider] = useState<string | null>(null)

  return (
    <div className="space-y-10">
      {/* ── Pipeline Flow ── */}
      <section>
        <SectionHeader
          title="Audio Pipeline"
          description="5-stage pipeline from microphone to transcription — click any stage to see its internals"
        />
        {/* Horizontal pipeline */}
        <div className="flex items-stretch gap-1 overflow-x-auto pb-2">
          {PIPELINE_STAGES.map((stage, i) => (
            <div key={stage.name} className="flex items-center">
              <NodeCard
                title={stage.name}
                icon={stage.icon}
                description={stage.tech}
                color={stage.color}
              >
                <div className="text-xs space-y-2">
                  <p className="text-slate-400">{stage.description}</p>
                  <p className="text-slate-500">{stage.details}</p>
                </div>
              </NodeCard>
              {i < PIPELINE_STAGES.length - 1 && (
                <div className="flex items-center px-1 shrink-0">
                  <div className="w-6 h-px bg-emerald-500/30" />
                  <span className="text-emerald-500/50 text-xs">→</span>
                </div>
              )}
            </div>
          ))}
        </div>
      </section>

      {/* ── Hybrid Transcription ── */}
      <section>
        <SectionHeader
          title="Hybrid Transcription"
          description="Three engines work together — VAD gates, Vosk gives instant feedback, Whisper delivers accuracy"
        />

        {/* Audio format spec box */}
        <div className="mb-6 p-3 rounded-lg bg-emerald-500/5 border border-emerald-500/20 flex flex-wrap gap-6 text-xs">
          <div><span className="text-emerald-400 font-medium">Sample Rate:</span> <span className="text-slate-300 font-mono">16,000 Hz</span></div>
          <div><span className="text-emerald-400 font-medium">Bit Depth:</span> <span className="text-slate-300 font-mono">16-bit signed int</span></div>
          <div><span className="text-emerald-400 font-medium">Channels:</span> <span className="text-slate-300 font-mono">Mono</span></div>
          <div><span className="text-emerald-400 font-medium">Window:</span> <span className="text-slate-300 font-mono">3s (0.5s overlap)</span></div>
        </div>

        {/* Provider cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          {TRANSCRIPTION_PROVIDERS.map((p) => (
            <button
              key={p.name}
              onClick={() => setActiveProvider(activeProvider === p.name ? null : p.name)}
              className={`text-left p-4 rounded-xl border transition-all ${
                activeProvider === p.name
                  ? 'border-emerald-500/40 bg-emerald-500/5 scale-[1.02]'
                  : 'border-slate-800 bg-slate-900/40 hover:border-slate-700'
              }`}
            >
              <div className="flex items-center gap-2 mb-2">
                <span className="text-xl">{p.icon}</span>
                <span className={`font-semibold text-sm ${p.color}`}>{p.name}</span>
              </div>
              <p className="text-xs text-slate-500 mb-2">{p.purpose}</p>
              <div className="flex items-center gap-2">
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-slate-800 font-mono text-emerald-400">
                  {p.timing}
                </span>
                <span className="text-[10px] text-slate-600">{p.tech}</span>
              </div>
              {activeProvider === p.name && (
                <p className="text-xs text-slate-400 mt-3 animate-fade-in">{p.description}</p>
              )}
            </button>
          ))}
        </div>

        {/* Flow diagram */}
        <div className="mt-6 p-4 rounded-lg bg-slate-900/40 border border-slate-800/50">
          <p className="text-xs text-slate-500 mb-3 font-medium">Transcription Flow</p>
          <div className="flex items-center gap-2 text-xs font-mono flex-wrap">
            <span className="px-2 py-1 rounded bg-emerald-500/10 text-emerald-400">Audio chunk</span>
            <span className="text-slate-600">→</span>
            <span className="px-2 py-1 rounded bg-emerald-500/10 text-emerald-300">Silero VAD</span>
            <span className="text-slate-600">→ speech?</span>
            <span className="text-slate-600">→</span>
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                <span className="px-2 py-1 rounded bg-slate-800 text-slate-400">Vosk</span>
                <span className="text-slate-600">→</span>
                <span className="px-2 py-1 rounded bg-slate-800 text-slate-500 italic">partial (gray)</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="px-2 py-1 rounded bg-slate-800 text-white">Whisper</span>
                <span className="text-slate-600">→</span>
                <span className="px-2 py-1 rounded bg-slate-800 text-white">final (white)</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* ── Sidecar Detail ── */}
      <section>
        <SectionHeader
          title="The Stdout Corruption Bug"
          description="Why JarvisListen writes to a file instead of stdout"
        />
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="p-4 rounded-lg border border-red-500/20 bg-red-500/5">
            <p className="text-xs text-red-400 font-medium mb-2">Before (broken)</p>
            <p className="text-xs text-slate-400 mb-3">
              JarvisListen wrote PCM to stdout. Tauri's shell plugin splits stdout on <code className="text-red-300">\n</code> bytes (0x0A).
              PCM audio naturally contains 0x0A bytes — so binary data got corrupted.
            </p>
            <CodeSnippet
              language="swift"
              code="// PCM data contains 0x0A bytes\n// Tauri splits on \\n → corrupted audio\nFileHandle.standardOutput.write(pcmData)"
            />
          </div>
          <div className="p-4 rounded-lg border border-emerald-500/20 bg-emerald-500/5">
            <p className="text-xs text-emerald-400 font-medium mb-2">After (fixed)</p>
            <p className="text-xs text-slate-400 mb-3">
              JarvisListen writes PCM to a file via <code className="text-emerald-300">--output</code> flag.
              No stdout involvement — binary data stays intact.
            </p>
            <CodeSnippet
              language="swift"
              code="// Write directly to file\n// --output ~/.jarvis/recordings/file.pcm\nlet handle = FileHandle(forWritingAtPath: path)\nhandle.write(pcmData)"
            />
          </div>
        </div>
      </section>

      <KeyFiles
        color="text-emerald-400"
        files={[
          { path: 'src-tauri/src/recording.rs', description: 'Recording lifecycle — spawn sidecar, manage files, create FIFO' },
          { path: 'src-tauri/src/transcription/manager.rs', description: 'Orchestrates hybrid pipeline — distributes audio to providers' },
          { path: 'src-tauri/src/transcription/hybrid_provider.rs', description: 'VAD → Vosk → Whisper coordination logic' },
          { path: 'src-tauri/src/transcription/audio_router.rs', description: 'Tails PCM file, chunks audio, sends via mpsc' },
          { path: 'src-tauri/src/transcription/vad.rs', description: 'Silero VAD — voice activity detection via ONNX runtime' },
          { path: 'jarvis-listen/', description: 'Swift CLI — ScreenCaptureKit audio capture' },
        ]}
      />
    </div>
  )
}
