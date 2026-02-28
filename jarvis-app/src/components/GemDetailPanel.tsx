import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gem, KnowledgeEntry } from '../state/types';

/** Shape of Co-Pilot data stored in gem's source_meta.copilot */
interface CoPilotGemData {
  summary?: string;
  key_points?: string[];
  decisions?: string[];
  action_items?: string[];
  open_questions?: string[];
  key_concepts?: Array<{ term: string; context: string }>;
  total_cycles?: number;
  total_audio_analyzed_seconds?: number;
}

interface GemDetailPanelProps {
  gemId: string;
  onDelete: () => void;
  onTranscribe: () => void;
  onEnrich: () => void;
  aiAvailable: boolean;
  onOpenKnowledgeFile: (filename: string) => void;
}

export default function GemDetailPanel({
  gemId,
  onDelete,
  onTranscribe,
  onEnrich,
  aiAvailable,
  onOpenKnowledgeFile
}: GemDetailPanelProps) {
  const [gem, setGem] = useState<Gem | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [knowledgeEntry, setKnowledgeEntry] = useState<KnowledgeEntry | null>(null);

  useEffect(() => {
    loadGem();
    loadKnowledge();
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

  const loadKnowledge = async () => {
    try {
      const entry = await invoke<KnowledgeEntry | null>('get_gem_knowledge', { gemId });
      setKnowledgeEntry(entry);
    } catch {
      // Silent fail — knowledge viewer is optional
      setKnowledgeEntry(null);
    }
  };

  const handleRegenerate = async () => {
    try {
      await invoke('regenerate_gem_knowledge', { gemId });
      await loadKnowledge();
    } catch (err) {
      console.error('Failed to regenerate knowledge:', err);
    }
  };

  const formatDate = (isoString: string) => {
    return new Date(isoString).toLocaleString();
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
  const copilot = gem.source_meta?.copilot as CoPilotGemData | undefined;

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

      {/* Co-Pilot Data (Requirement 10.3, 10.4, 10.5, 10.6) */}
      {copilot && (
        <div className="gem-copilot-section">
          <h4 className="copilot-section-title">Co-Pilot Analysis</h4>
          
          {copilot.summary && (
            <div className="copilot-summary">
              <h5>Summary</h5>
              <p>{copilot.summary}</p>
            </div>
          )}

          {copilot.key_points && copilot.key_points.length > 0 && (
            <div className="copilot-key-points">
              <h5>Key Points</h5>
              <ul>
                {copilot.key_points.map((point: string, i: number) => (
                  <li key={i}>{point}</li>
                ))}
              </ul>
            </div>
          )}

          {copilot.decisions && copilot.decisions.length > 0 && (
            <div className="copilot-decisions">
              <h5>Decisions</h5>
              <ul>
                {copilot.decisions.map((decision: string, i: number) => (
                  <li key={i}>
                    <span className="decision-icon">✓</span>
                    {decision}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {copilot.action_items && copilot.action_items.length > 0 && (
            <div className="copilot-action-items">
              <h5>Action Items</h5>
              <ul>
                {copilot.action_items.map((item: string, i: number) => (
                  <li key={i}>{item}</li>
                ))}
              </ul>
            </div>
          )}

          {copilot.open_questions && copilot.open_questions.length > 0 && (
            <div className="copilot-open-questions">
              <h5>Open Questions</h5>
              <ul>
                {copilot.open_questions.map((question: string, i: number) => (
                  <li key={i}>{question}</li>
                ))}
              </ul>
            </div>
          )}

          {copilot.key_concepts && copilot.key_concepts.length > 0 && (
            <div className="copilot-key-concepts">
              <h5>Key Concepts</h5>
              <div className="concepts-grid">
                {copilot.key_concepts.map((concept: any, i: number) => (
                  <div key={i} className="concept-chip" title={concept.context}>
                    {concept.term}
                  </div>
                ))}
              </div>
            </div>
          )}

          {copilot.total_cycles && (
            <div className="copilot-metadata">
              <span className="metadata-label">Analysis cycles:</span>
              <span className="metadata-value">{copilot.total_cycles}</span>
              {copilot.total_audio_analyzed_seconds && (
                <>
                  <span className="metadata-separator">•</span>
                  <span className="metadata-label">Audio analyzed:</span>
                  <span className="metadata-value">
                    {Math.floor(copilot.total_audio_analyzed_seconds / 60)}m {copilot.total_audio_analyzed_seconds % 60}s
                  </span>
                </>
              )}
            </div>
          )}
        </div>
      )}

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

      {/* Knowledge Files Section */}
      {knowledgeEntry && (() => {
        const existingFiles = knowledgeEntry.subfiles.filter(
          s => s.exists && s.filename !== 'meta.json'
        );
        const fileOrder = ['content.md', 'enrichment.md', 'transcript.md', 'copilot.md', 'gem.md'];
        const sortedFiles = existingFiles.sort((a, b) => {
          const aIndex = fileOrder.indexOf(a.filename);
          const bIndex = fileOrder.indexOf(b.filename);
          return (aIndex === -1 ? 999 : aIndex) - (bIndex === -1 ? 999 : bIndex);
        });

        if (sortedFiles.length === 0) {
          return (
            <div className="knowledge-file-tree">
              <h4>Knowledge Files</h4>
              <div className="no-knowledge-files">
                <span>No knowledge files</span>
                <button onClick={handleRegenerate} className="action-button">
                  Generate
                </button>
              </div>
            </div>
          );
        }

        return (
          <div className="knowledge-file-tree">
            <h4>Knowledge Files</h4>
            {sortedFiles.map(subfile => (
              <div
                key={subfile.filename}
                className="knowledge-file-row"
                onClick={() => onOpenKnowledgeFile(subfile.filename)}
              >
                <span className="file-icon">📄</span>
                <span className="file-name">{subfile.filename}</span>
                <span className="file-size">{formatFileSize(subfile.size_bytes)}</span>
              </div>
            ))}
          </div>
        );
      })()}

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
