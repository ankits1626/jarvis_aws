import { useEffect, useRef, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptionSegment, TranscriptionStatus, PageGist, Gem } from '../state/types';

interface TranscriptDisplayProps {
  transcript: TranscriptionSegment[];
  status: TranscriptionStatus;
  error: string | null;
  recordingFilename?: string | null;
}

/**
 * Process transcript to replace Vosk partials with Whisper finals.
 *
 * Each audio window emits: Vosk partial (is_final=false) → Whisper final (is_final=true).
 * When a final arrives, it replaces the immediately preceding partial.
 */
function processTranscript(segments: TranscriptionSegment[]): TranscriptionSegment[] {
  const result: TranscriptionSegment[] = [];

  for (const segment of segments) {
    if (segment.is_final) {
      // Remove the last partial — it's from the same audio window
      if (result.length > 0 && !result[result.length - 1].is_final) {
        result.pop();
      }
      result.push(segment);
    } else {
      result.push(segment);
    }
  }

  return result;
}

export function TranscriptDisplay({ transcript, status, error, recordingFilename }: TranscriptDisplayProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Process transcript to handle partial → final replacement
  const displaySegments = useMemo(() => processTranscript(transcript), [transcript]);

  // Auto-scroll to latest text when transcript updates
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [displaySegments]);

  const handleSaveGem = async () => {
    setSaving(true);
    setSaveError(null);

    try {
      // Join all final segments into full transcript text
      const fullText = displaySegments
        .filter(s => s.is_final)
        .map(s => s.text)
        .join(' ');

      // Construct PageGist object
      const gist: PageGist = {
        url: `jarvis://recording/${Date.now()}`,
        title: `Audio Transcript – ${new Date().toLocaleString()}`,
        source_type: 'Other',
        domain: 'jarvis-app',
        author: null,
        description: null,
        content_excerpt: fullText,
        published_date: new Date().toISOString(),
        image_url: null,
        extra: {
          segment_count: displaySegments.filter(s => s.is_final).length,
          source: 'audio_transcription',
          recording_filename: recordingFilename || null,
        },
      };

      await invoke<Gem>('save_gem', { gist });
      setSaved(true);
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const hasFinalSegments = displaySegments.filter(s => s.is_final).length > 0;

  if (status === 'disabled') {
    return null; // Don't show component if transcription is disabled
  }

  return (
    <div className="transcript-display">
      <div className="transcript-header">
        <h3>Live Transcript</h3>
        {status === 'active' && (
          <span className="transcribing-indicator">
            <span className="pulse-dot"></span>
            Transcribing...
          </span>
        )}
        {status === 'error' && error && (
          <span className="error-indicator">⚠️ {error}</span>
        )}
        {hasFinalSegments && (
          <button
            onClick={handleSaveGem}
            className="save-gem-button"
            disabled={saved || saving}
            style={{ marginLeft: '10px' }}
          >
            {saved ? 'Saved' : saving ? 'Saving...' : 'Save Gem'}
          </button>
        )}
      </div>

      {saveError && (
        <div className="error-state" style={{ marginBottom: '12px' }}>
          {saveError}
        </div>
      )}

      <div className="transcript-content" ref={containerRef}>
        {displaySegments.length === 0 && status === 'idle' && (
          <p className="empty-transcript">
            Start recording to see live transcription
          </p>
        )}

        {displaySegments.map((segment, index) => (
          <span
            key={`${segment.start_ms}-${index}`}
            className={segment.is_final ? 'segment-final' : 'segment-partial'}
          >
            {segment.text}{' '}
          </span>
        ))}
      </div>
    </div>
  );
}
