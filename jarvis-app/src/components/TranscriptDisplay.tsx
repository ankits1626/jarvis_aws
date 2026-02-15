import { useEffect, useRef, useMemo } from 'react';
import type { TranscriptionSegment, TranscriptionStatus } from '../state/types';

interface TranscriptDisplayProps {
  transcript: TranscriptionSegment[];
  status: TranscriptionStatus;
  error: string | null;
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

export function TranscriptDisplay({ transcript, status, error }: TranscriptDisplayProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Process transcript to handle partial → final replacement
  const displaySegments = useMemo(() => processTranscript(transcript), [transcript]);

  // Auto-scroll to latest text when transcript updates
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [displaySegments]);

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
      </div>

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
