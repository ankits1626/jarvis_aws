import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { YouTubeGist, YouTubeDetectedEvent } from '../state/types';

interface DetectedVideo extends YouTubeDetectedEvent {
  gist?: YouTubeGist;
  loading?: boolean;
  error?: string;
}

interface YouTubeSectionProps {
  onClose?: () => void;
  detectedVideos: YouTubeDetectedEvent[];
  onDismissVideo: (videoId: string) => void;
}

interface VideoCardProps {
  video: DetectedVideo;
  onPrepareGist: () => void;
  onDismiss: () => void;
  onCopy: () => void;
}

function VideoCard({ video, onPrepareGist, onDismiss, onCopy }: VideoCardProps) {
  const formatDuration = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="video-card">
      <div className="video-url">{video.title || video.url}</div>
      {video.author_name && (
        <div className="video-author">by {video.author_name}</div>
      )}
      
      {!video.gist && !video.loading && !video.error && (
        <button onClick={onPrepareGist} className="prepare-gist-button">
          Prepare Gist
        </button>
      )}
      
      {video.loading && (
        <div className="loading-state">Loading gist...</div>
      )}
      
      {video.error && (
        <div className="error-state">Error: {video.error}</div>
      )}
      
      {video.gist && (
        <div className="gist-display">
          <div className="gist-header">Gist of {video.url}</div>
          <div className="gist-field">
            <span className="gist-label">Title:</span> {video.gist.title}
          </div>
          <div className="gist-field">
            <span className="gist-label">Channel:</span> {video.gist.channel}
          </div>
          <div className="gist-field">
            <span className="gist-label">Duration:</span> {formatDuration(video.gist.duration_seconds)}
          </div>
          <div className="gist-description">
            <div className="gist-label">Description:</div>
            <div className="gist-description-text">{video.gist.description}</div>
          </div>
          <div className="gist-actions">
            <button onClick={onCopy} className="copy-button">Copy</button>
            <button onClick={onDismiss} className="gist-dismiss-button">Dismiss</button>
          </div>
        </div>
      )}
    </div>
  );
}

export function YouTubeSection({ onClose, detectedVideos, onDismissVideo }: YouTubeSectionProps) {
  // Local state extends detected videos with gist/loading/error state
  const [videos, setVideos] = useState<DetectedVideo[]>([]);

  // Sync incoming detected videos into local state (preserving gist data)
  useEffect(() => {
    setVideos(prev => {
      const existingById = new Map(prev.map(v => [v.video_id, v]));
      const merged = detectedVideos.map(dv => {
        const existing = existingById.get(dv.video_id);
        if (existing) return { ...existing, ...dv, gist: existing.gist, loading: existing.loading, error: existing.error };
        return { ...dv };
      });
      return merged;
    });
  }, [detectedVideos]);

  const handlePrepareGist = async (index: number) => {
    const video = videos[index];
    setVideos(prev => prev.map((v, i) => 
      i === index ? { ...v, loading: true } : v
    ));
    
    try {
      const gist = await invoke<YouTubeGist>('fetch_youtube_gist', { 
        url: video.url 
      });
      setVideos(prev => prev.map((v, i) => 
        i === index ? { ...v, gist, loading: false } : v
      ));
    } catch (err) {
      setVideos(prev => prev.map((v, i) => 
        i === index ? { ...v, error: String(err), loading: false } : v
      ));
    }
  };

  const handleDismiss = (index: number) => {
    const video = videos[index];
    setVideos(prev => prev.filter((_, i) => i !== index));
    onDismissVideo(video.video_id);
  };

  const formatGist = (gist: YouTubeGist): string => {
    const minutes = Math.floor(gist.duration_seconds / 60);
    const seconds = gist.duration_seconds % 60;
    const duration = `${minutes}:${seconds.toString().padStart(2, '0')}`;
    
    return `Gist of ${gist.url}

Title: ${gist.title}
Channel: ${gist.channel}
Duration: ${duration}

Description:
${gist.description}`;
  };

  const handleCopy = (gist: YouTubeGist) => {
    const text = formatGist(gist);
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>YouTube</h2>
        {onClose && <button onClick={onClose} className="close-button">×</button>}
      </div>
      <div className="settings-content">
        <div className="videos-list">
          {videos.length === 0 && (
            <p className="empty-state">No YouTube videos detected yet</p>
          )}
          {videos.map((video, index) => (
            <VideoCard
              key={video.video_id}
              video={video}
              onPrepareGist={() => handlePrepareGist(index)}
              onDismiss={() => handleDismiss(index)}
              onCopy={() => video.gist && handleCopy(video.gist)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
