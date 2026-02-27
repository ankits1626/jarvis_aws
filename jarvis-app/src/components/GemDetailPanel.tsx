import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gem } from '../state/types';

interface GemDetailPanelProps {
  gemId: string;
  onDelete: () => void;
  onTranscribe: () => void;
  onEnrich: () => void;
  aiAvailable: boolean;
}

export default function GemDetailPanel({
  gemId,
  onDelete,
  onTranscribe,
  onEnrich,
  aiAvailable
}: GemDetailPanelProps) {
  const [gem, setGem] = useState<Gem | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadGem();
  }, [gemId]);

  const loadGem = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<Gem>('get_gem', { id: gemId });
      setGem(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load gem');
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (isoString: string) => {
    return new Date(isoString).toLocaleString();
  };

  if (loading) {
    return (
      <div className="gem-detail-panel">
        <div className="loading-state">Loading gem...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="gem-detail-panel">
        <div className="error-state">
          <p>{error}</p>
          <button onClick={loadGem} className="retry-button">
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!gem) {
    return (
      <div className="gem-detail-panel">
        <div className="error-state">Gem not found</div>
      </div>
    );
  }

  const isAudioGem = gem.source_url.startsWith('jarvis://recording/');

  return (
    <div className="gem-detail-panel">
      <div className="gem-detail-header">
        <div className="gem-title-section">
          <h3>{gem.title}</h3>
          <span className="source-badge">{gem.source_type}</span>
        </div>
      </div>

      <div className="gem-detail-metadata">
        <div className="metadata-item">
          <span className="metadata-label">Domain:</span>
          <span className="metadata-value">{gem.domain}</span>
        </div>
        {gem.author && (
          <div className="metadata-item">
            <span className="metadata-label">Author:</span>
            <span className="metadata-value">{gem.author}</span>
          </div>
        )}
        <div className="metadata-item">
          <span className="metadata-label">Captured:</span>
          <span className="metadata-value">{formatDate(gem.captured_at)}</span>
        </div>
      </div>

      {gem.ai_enrichment?.tags && gem.ai_enrichment.tags.length > 0 && (
        <div className="gem-tags">
          {gem.ai_enrichment.tags.map((tag, index) => (
            <span key={index} className="gem-detail-tag">
              {tag}
            </span>
          ))}
        </div>
      )}

      {gem.ai_enrichment?.summary && (
        <div className="gem-summary">
          <h4>Summary</h4>
          <p>{gem.ai_enrichment.summary}</p>
        </div>
      )}

      {gem.transcript && (
        <div className="gem-transcript">
          <div className="transcript-header">
            <h4>Transcript</h4>
            {gem.transcript_language && (
              <span className="language-indicator">
                {gem.transcript_language.toUpperCase()}
              </span>
            )}
          </div>
          <div className="transcript-text scrollable">
            {gem.transcript}
          </div>
        </div>
      )}

      <div className="gem-actions">
        {isAudioGem && (
          <button onClick={onTranscribe} className="action-button">
            Transcribe
          </button>
        )}
        {aiAvailable && (
          <button onClick={onEnrich} className="action-button">
            Enrich
          </button>
        )}
        <button onClick={onDelete} className="action-button delete-button">
          Delete
        </button>
      </div>
    </div>
  );
}
