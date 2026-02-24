import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { GemPreview, Gem } from '../state/types';

interface GemsPanelProps {
  onClose: () => void;
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

function GemCard({ gem, onDelete }: { gem: GemPreview; onDelete: (id: string) => Promise<void> }) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [fullGem, setFullGem] = useState<Gem | null>(null);
  const [loading, setLoading] = useState(false);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [audioLoading, setAudioLoading] = useState(false);
  const [audioError, setAudioError] = useState<string | null>(null);

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
    <div className="gem-card">
      <div className="gem-card-header">
        <span className={badgeClass}>{gem.source_type}</span>
        <span className="gem-date">
          {new Date(gem.captured_at).toLocaleDateString()}
        </span>
      </div>
      <div className="gem-title">{gem.title}</div>
      <div className="gem-meta">
        <span className="gem-domain">{gem.domain}</span>
        {gem.author && <span className="gem-author">by {gem.author}</span>}
      </div>
      {gem.description && (
        <div className="gem-description">{gem.description}</div>
      )}
      {!expanded && gem.content_preview && (
        <div className="gem-preview">{gem.content_preview}</div>
      )}
      
      {expanded && fullGem && (
        <div className="gem-expanded-content">
          {fullGem.content && (
            <div className="gem-full-content">
              <div className="gem-content-label">Full Content:</div>
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
      
      <div className="gem-actions">
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
    </div>
  );
}

export function GemsPanel({ onClose }: GemsPanelProps) {
  const [gems, setGems] = useState<GemPreview[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [error, setError] = useState<string | null>(null);

  const fetchGems = useCallback(async (query: string) => {
    setLoading(true);
    setError(null);
    try {
      const results = query.trim()
        ? await invoke<GemPreview[]>('search_gems', { query })
        : await invoke<GemPreview[]>('list_gems', {});
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
      fetchGems(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, fetchGems]);

  const handleDelete = async (id: string) => {
    try {
      await invoke('delete_gem', { id });
      setGems(prev => prev.filter(g => g.id !== id));
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>Gems</h2>
        <button onClick={onClose} className="close-button">×</button>
      </div>
      <div className="settings-content">
        <div className="gems-search">
          <input
            type="search"
            placeholder="Search gems..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="gems-search-input"
          />
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
            <GemCard key={gem.id} gem={gem} onDelete={handleDelete} />
          ))}
        </div>
      </div>
    </div>
  );
}
