import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface AudioPlayerProps {
  filename: string | null;
  onClose: () => void;
}

export function AudioPlayer({ filename, onClose }: AudioPlayerProps) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!filename) {
      return;
    }

    let currentBlobUrl: string | null = null;

    const loadAudio = async () => {
      setLoading(true);
      setError(null);

      try {
        const wavBytes = await invoke<number[]>('convert_to_wav', { filename });
        const uint8Array = new Uint8Array(wavBytes);
        const blob = new Blob([uint8Array], { type: 'audio/wav' });
        currentBlobUrl = URL.createObjectURL(blob);
        setBlobUrl(currentBlobUrl);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    loadAudio();

    return () => {
      if (currentBlobUrl) {
        URL.revokeObjectURL(currentBlobUrl);
      }
    };
  }, [filename]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    const handleEnded = () => {
      audio.currentTime = 0;
    };

    audio.addEventListener('ended', handleEnded);
    return () => {
      audio.removeEventListener('ended', handleEnded);
    };
  }, []);

  if (!filename) {
    return null;
  }

  return (
    <div className="audio-player">
      <div className="audio-player-header">
        <h3>Playing: {filename}</h3>
        <button
          className="close-button"
          onClick={onClose}
          aria-label="Close audio player"
        >
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
          </svg>
        </button>
      </div>

      {loading && <div className="audio-player-loading">Loading...</div>}

      {error && (
        <div className="audio-player-error">
          Error loading audio: {error}
        </div>
      )}

      {blobUrl && !loading && !error && (
        <audio
          ref={audioRef}
          src={blobUrl}
          controls
          className="audio-controls"
        />
      )}
    </div>
  );
}
