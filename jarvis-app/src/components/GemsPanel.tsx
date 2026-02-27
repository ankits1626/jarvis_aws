import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { GemPreview, Gem, AvailabilityResult } from '../state/types';

interface GemsPanelProps {
  onClose?: () => void;
  onGemSelect?: (gemId: string | null) => void;
}

const SOURCE_BADGE_CLASS: Record<string, string> = {
  YouTube: 'source-badge youtube',
  Article: 'source-badge article',
  Code: 'source-badge code',
  Docs: 'source-badge docs',
  Email: 'source-badge email',
  Chat: 'source-badge chat',
  QA: 'source-badge qa',
  News: 'source-badge news',
  Research: 'source-badge research',
  Social: 'source-badge social',
  Other: 'source-badge other',
};

function GemCard({ 
  gem, 
  onDelete, 
  aiAvailable,
  onFilterByTag,
  onSelect
}: { 
  gem: GemPreview; 
  onDelete: (id: string) => Promise<void>;
  aiAvailable: boolean;
  onFilterByTag: (tag: string) => void;
  onSelect?: (gemId: string) => void;
}) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [fullGem, setFullGem] = useState<Gem | null>(null);
  const [loading, setLoading] = useState(false);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [audioLoading, setAudioLoading] = useState(false);
  const [audioError, setAudioError] = useState<string | null>(null);
  const [enriching, setEnriching] = useState(false);
  const [enrichError, setEnrichError] = useState<string | null>(null);
  const [transcribing, setTranscribing] = useState(false);
  const [transcribeError, setTranscribeError] = useState<string | null>(null);
  const [localGem, setLocalGem] = useState<GemPreview>(gem);

  // Update local gem when prop changes (e.g., after enrichment)
  useEffect(() => {
    setLocalGem(gem);
  }, [gem]);

  const handleEnrich = async () => {
    setEnriching(true);
    setEnrichError(null);
    try {
      const enrichedGem = await invoke<Gem>('enrich_gem', { id: localGem.id });
      // Update local state with new tags, summary, and enrichment source
      const provider = enrichedGem.ai_enrichment?.provider;
      const model = enrichedGem.ai_enrichment?.model;
      const source = provider && model ? `${provider} / ${model}` : provider || null;
      setLocalGem({
        ...localGem,
        tags: enrichedGem.ai_enrichment?.tags || null,
        summary: enrichedGem.ai_enrichment?.summary || null,
        enrichment_source: source,
      });
      // Also update fullGem if it's cached
      if (fullGem) {
        setFullGem(enrichedGem);
      }
    } catch (err) {
      setEnrichError(String(err));
    } finally {
      setEnriching(false);
    }
  };

  const handleTranscribe = async () => {
    setTranscribing(true);
    setTranscribeError(null);
    try {
      const updatedGem = await invoke<Gem>('transcribe_gem', { id: localGem.id });
      // Update local state with transcript + regenerated tags/summary
      const provider = updatedGem.ai_enrichment?.provider;
      const model = updatedGem.ai_enrichment?.model;
      const source = provider && model ? `${provider} / ${model}` : provider || null;
      setLocalGem({
        ...localGem,
        transcript_language: updatedGem.transcript_language,
        tags: updatedGem.ai_enrichment?.tags || localGem.tags,
        summary: updatedGem.ai_enrichment?.summary || localGem.summary,
        enrichment_source: source || localGem.enrichment_source,
      });
      if (fullGem) {
        setFullGem(updatedGem);
      }
    } catch (err) {
      setTranscribeError(String(err));
    } finally {
      setTranscribing(false);
    }
  };

  const handleDelete = async () => {
    setDeleting(true);
    await onDelete(gem.id);
    setDeleting(false);
  };

  // Detect if this is an audio transcript gem
  const isAudioTranscript = gem.domain === 'jarvis-app';

  const handleOpen = async () => {
    if (isAudioTranscript) {
      // Audio transcript gem - play audio inline
      if (audioUrl) {
        // Stop playing - clean up audio URL
        URL.revokeObjectURL(audioUrl);
        setAudioUrl(null);
      } else {
        // Start playing - fetch recording and convert to WAV
        setAudioLoading(true);
        setAudioError(null);
        try {
          // Fetch full gem to get recording_filename from source_meta
          const fullGemData = fullGem || await invoke<Gem>('get_gem', { id: gem.id });
          if (!fullGem) setFullGem(fullGemData);
          
          const recordingFilename = fullGemData.source_meta?.recording_filename as string | undefined;
          if (!recordingFilename) {
            setAudioError('No recording file associated with this gem');
            return;
          }

          // Convert PCM to WAV
          const wavBytes = await invoke<number[]>('convert_to_wav', { filename: recordingFilename });
          const blob = new Blob([new Uint8Array(wavBytes)], { type: 'audio/wav' });
          const url = URL.createObjectURL(blob);
          setAudioUrl(url);
        } catch (err) {
          setAudioError('Failed to load audio');
        } finally {
          setAudioLoading(false);
        }
      }
    } else {
      // Regular gem - open URL in browser
      try {
        await open(gem.source_url);
      } catch (err) {
        console.error('Failed to open URL:', err);
      }
    }
  };

  const handleToggleExpand = async () => {
    if (expanded) {
      // Collapse
      setExpanded(false);
    } else {
      // Expand - fetch full gem if not already cached
      if (!fullGem) {
        setLoading(true);
        try {
          const result = await invoke<Gem>('get_gem', { id: gem.id });
          setFullGem(result);
          setExpanded(true); // Only expand on successful fetch
        } catch (err) {
          console.error('Failed to fetch full gem:', err);
          // Don't expand on failure - user can retry via "View" button
        } finally {
          setLoading(false);
        }
      } else {
        // Already cached, safe to expand
        setExpanded(true);
      }
    }
  };

  const badgeClass = SOURCE_BADGE_CLASS[gem.source_type] || 'source-badge other';

  return (
    <div className="gem-card" onClick={() => onSelect?.(gem.id)} style={{ cursor: onSelect ? 'pointer' : 'default' }}>
      <div className="gem-card-header">
        <span className={badgeClass}>{gem.source_type}</span>
        <span className="gem-date">
          {new Date(gem.captured_at).toLocaleDateString()}
        </span>
      </div>
      <div className="gem-title">
        {gem.title}
      </div>
      <div className="gem-meta">
        <span className="gem-domain">{gem.domain}</span>
        {gem.author && <span className="gem-author">by {gem.author}</span>}
        {isAudioTranscript && localGem.transcript_language && (
          <span className="gem-lang-badge" title="Transcript available">
            {localGem.transcript_language}
          </span>
        )}
      </div>
      {gem.description && (
        <div className="gem-description">{gem.description}</div>
      )}
      
      {localGem.tags && localGem.tags.length > 0 && (
        <div className="gem-tags">
          {localGem.tags.map((tag, idx) => (
            <button
              key={idx}
              className="gem-tag"
              onClick={(e) => { e.stopPropagation(); onFilterByTag(tag); }}
              title={`Filter by tag: ${tag}`}
            >
              {tag}
            </button>
          ))}
        </div>
      )}
      
      {localGem.summary && (
        <div className="gem-summary">{localGem.summary}</div>
      )}

      {(enriching || transcribing) && (
        <div className="gem-enriching" style={{
          padding: '8px 12px',
          marginTop: '8px',
          backgroundColor: '#fff3cd',
          border: '1px solid #ffc107',
          borderRadius: '4px',
          color: '#856404',
          fontSize: '13px'
        }}>
          {transcribing ? 'Transcribing audio...' : 'Enriching with AI...'}
        </div>
      )}

      {localGem.enrichment_source && (localGem.tags || localGem.summary) && (
        <div className="gem-enrichment-source">
          Enriched by: {localGem.enrichment_source}
        </div>
      )}

      {!expanded && gem.content_preview && (
        <div className="gem-preview">{gem.content_preview}</div>
      )}
      
      {expanded && fullGem && (
        <div className="gem-expanded-content">
          {/* MLX Omni Transcript (if available) */}
          {fullGem.transcript && (
            <div className="gem-transcript">
              <div className="gem-content-label">
                Transcript {fullGem.transcript_language && `(${fullGem.transcript_language})`}
              </div>
              <div className="gem-content-text">{fullGem.transcript}</div>
            </div>
          )}
          
          {/* Whisper Transcript (collapsed if MLX transcript exists) */}
          {fullGem.content && (
            <div className="gem-full-content">
              <div className="gem-content-label">
                {fullGem.transcript ? '▼ Real-time Transcript (Whisper)' : 'Transcript (Whisper)'}
              </div>
              <div className="gem-content-text">{fullGem.content}</div>
            </div>
          )}
          
          {fullGem.source_meta && (
            <div className="gem-source-meta">
              <div className="gem-content-label">Metadata:</div>
              <pre className="gem-meta-text">{JSON.stringify(fullGem.source_meta, null, 2)}</pre>
            </div>
          )}
        </div>
      )}
      
      <div className="gem-actions" onClick={(e) => e.stopPropagation()}>
        <button 
          onClick={handleOpen} 
          className="gem-open-button"
          disabled={audioLoading}
        >
          {audioLoading ? '...' : isAudioTranscript ? (audioUrl ? 'Stop' : 'Play') : 'Open'}
        </button>
        <button 
          onClick={handleToggleExpand} 
          className="gem-view-button"
          disabled={loading}
        >
          {loading ? '...' : expanded ? 'Collapse' : 'View'}
        </button>
        {aiAvailable && (
          <button
            onClick={handleEnrich}
            className="gem-enrich-button"
            disabled={enriching}
            title={localGem.tags ? 'Re-enrich with AI' : 'Enrich with AI'}
          >
            {enriching ? '...' : localGem.tags ? '🔄' : '✨'}
          </button>
        )}
        {!aiAvailable && (
          <button
            className="gem-enrich-button"
            disabled
            title="AI enrichment unavailable. Check Settings to configure an intelligence provider."
          >
            ✨
          </button>
        )}
        {isAudioTranscript && aiAvailable && !localGem.transcript_language && (
          <button
            onClick={handleTranscribe}
            className="gem-enrich-button"
            disabled={transcribing}
            title="Transcribe recording"
          >
            {transcribing ? '...' : 'Transcribe'}
          </button>
        )}
        {confirmDelete ? (
          <div className="gem-delete-confirm">
            <span>Delete?</span>
            <button
              onClick={handleDelete}
              className="gem-confirm-yes"
              disabled={deleting}
            >
              {deleting ? '...' : 'Yes'}
            </button>
            <button
              onClick={() => setConfirmDelete(false)}
              className="gem-confirm-no"
            >
              No
            </button>
          </div>
        ) : (
          <button onClick={() => setConfirmDelete(true)} className="gem-delete-button">
            Delete
          </button>
        )}
      </div>
      
      {audioUrl && (
        <div className="gem-audio-player">
          <audio 
            controls 
            src={audioUrl} 
            autoPlay
            onEnded={() => {
              URL.revokeObjectURL(audioUrl);
              setAudioUrl(null);
            }}
          />
        </div>
      )}
      
      {audioError && (
        <div className="error-state" style={{ marginTop: '8px' }}>
          {audioError}
        </div>
      )}
      
      {enrichError && (
        <div className="error-state" style={{ marginTop: '8px' }}>
          {enrichError}
        </div>
      )}

      {transcribeError && (
        <div className="error-state" style={{ marginTop: '8px' }}>
          {transcribeError}
        </div>
      )}
    </div>
  );
}

export function GemsPanel({ onClose, onGemSelect }: GemsPanelProps) {
  const [gems, setGems] = useState<GemPreview[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [aiAvailability, setAiAvailability] = useState<AvailabilityResult | null>(null);
  const [filterTag, setFilterTag] = useState<string | null>(null);

  // Check AI availability on mount
  useEffect(() => {
    const checkAvailability = async () => {
      try {
        const result = await invoke<AvailabilityResult>('check_intel_availability');
        setAiAvailability(result);
      } catch (err) {
        console.error('Failed to check AI availability:', err);
        setAiAvailability({ available: false, reason: 'Failed to check' });
      }
    };
    checkAvailability();
  }, []);

  const fetchGems = useCallback(async (query: string, tag: string | null) => {
    setLoading(true);
    setError(null);
    try {
      let results: GemPreview[];
      if (tag) {
        // Filter by tag
        results = await invoke<GemPreview[]>('filter_gems_by_tag', { tag });
      } else if (query.trim()) {
        // Search by query
        results = await invoke<GemPreview[]>('search_gems', { query });
      } else {
        // List all
        results = await invoke<GemPreview[]>('list_gems', {});
      }
      setGems(results);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  // Debounced search (300ms) - also handles initial load since searchQuery starts as ''
  useEffect(() => {
    const timer = setTimeout(() => {
      fetchGems(searchQuery, filterTag);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, filterTag, fetchGems]);

  const handleFilterByTag = (tag: string) => {
    setFilterTag(tag);
    setSearchQuery(''); // Clear search when filtering by tag
  };

  const handleClearFilter = () => {
    setFilterTag(null);
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke('delete_gem', { id });
      setGems(prev => prev.filter(g => g.id !== id));
      // Clear right panel selection if this gem was selected
      onGemSelect?.(null);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>
          Gems
          {aiAvailability && (
            <span
              className={`ai-badge ${aiAvailability.available ? 'available' : 'unavailable'}`}
              title={
                aiAvailability.available
                  ? 'AI enrichment available'
                  : `AI enrichment unavailable: ${aiAvailability.reason || 'Unknown reason'}`
              }
            >
              AI
            </span>
          )}
        </h2>
        {onClose && <button onClick={onClose} className="close-button">×</button>}
      </div>
      <div className="settings-content">
        {aiAvailability && !aiAvailability.available && (
          <div className="info-banner" style={{ 
            padding: '12px', 
            marginBottom: '16px', 
            backgroundColor: '#fff3cd', 
            border: '1px solid #ffc107',
            borderRadius: '4px',
            color: '#856404'
          }}>
            <strong>AI enrichment unavailable:</strong> {aiAvailability.reason || 'Unknown reason'}
            <br />
            <small>Configure an intelligence provider in Settings to enable AI-powered tags and summaries.</small>
          </div>
        )}
        <div className="gems-search">
          <input
            type="search"
            placeholder="Search gems..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="gems-search-input"
            disabled={!!filterTag}
          />
          {filterTag && (
            <div className="active-filter">
              Filtering by tag: <strong>{filterTag}</strong>
              <button onClick={handleClearFilter} className="clear-filter-button">
                ×
              </button>
            </div>
          )}
        </div>

        {error && (
          <div className="error-state" style={{ marginBottom: '12px' }}>
            {error}
          </div>
        )}

        {loading && gems.length === 0 && (
          <div className="loading-state">Loading gems...</div>
        )}

        {!loading && gems.length === 0 && (
          <div className="empty-state">
            {searchQuery.trim()
              ? 'No gems match your search.'
              : 'No gems yet. Extract a gist from the Browser tool and save it.'}
          </div>
        )}

        <div className="gems-list">
          {gems.map(gem => (
            <GemCard 
              key={gem.id} 
              gem={gem} 
              onDelete={handleDelete}
              aiAvailable={aiAvailability?.available || false}
              onFilterByTag={handleFilterByTag}
              onSelect={onGemSelect}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
