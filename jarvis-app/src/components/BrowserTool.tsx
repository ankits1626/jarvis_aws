import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { BrowserTab, PageGist, SourceType } from '../state/types';

interface BrowserToolProps {
  onClose: () => void;
}

const SOURCE_BADGES: Record<SourceType, { label: string; className: string }> = {
  YouTube: { label: 'YT', className: 'source-badge youtube' },
  Article: { label: 'Article', className: 'source-badge article' },
  Code: { label: 'Code', className: 'source-badge code' },
  Docs: { label: 'Docs', className: 'source-badge docs' },
  Email: { label: 'Email', className: 'source-badge email' },
  Chat: { label: 'Chat', className: 'source-badge chat' },
  QA: { label: 'Q&A', className: 'source-badge qa' },
  News: { label: 'News', className: 'source-badge news' },
  Research: { label: 'Research', className: 'source-badge research' },
  Social: { label: 'Social', className: 'source-badge social' },
  Other: { label: 'Other', className: 'source-badge other' },
};

function GistCard({ gist, onCopy, onExport, onDismiss }: { gist: PageGist; onCopy: () => void; onExport: () => void; onDismiss: () => void }) {
  const durationSeconds = gist.extra?.duration_seconds as number | undefined;
  const formatDuration = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="gist-display">
      <div className="gist-header">
        Gist of {gist.domain} · {gist.source_type}
        {durationSeconds ? ` · ${formatDuration(durationSeconds)}` : ''}
      </div>
      <div className="gist-field">
        <span className="gist-label">Title:</span> {gist.title}
      </div>
      {gist.author && (
        <div className="gist-field">
          <span className="gist-label">Author:</span> {gist.author}
        </div>
      )}
      {gist.published_date && (
        <div className="gist-field">
          <span className="gist-label">Published:</span> {gist.published_date}
        </div>
      )}
      {gist.description && (
        <div className="gist-description">
          <div className="gist-label">Description:</div>
          <div className="gist-description-text">{gist.description}</div>
        </div>
      )}
      {gist.content_excerpt && (
        <div className="gist-description">
          <div className="gist-label">Article:</div>
          <div className="gist-description-text" style={{ whiteSpace: 'pre-wrap' }}>{gist.content_excerpt}</div>
        </div>
      )}
      <div className="gist-actions">
        <button onClick={onCopy} className="copy-button">Copy</button>
        <button onClick={onExport} className="copy-button">Export</button>
        <button onClick={onDismiss} className="gist-dismiss-button">Dismiss</button>
      </div>
    </div>
  );
}

export function BrowserTool({ onClose }: BrowserToolProps) {
  const [tabs, setTabs] = useState<BrowserTab[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [gist, setGist] = useState<PageGist | null>(null);
  const [gistLoading, setGistLoading] = useState(false);
  const [gistError, setGistError] = useState<string | null>(null);
  const [exportStatus, setExportStatus] = useState<string | null>(null);

  const fetchTabs = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<BrowserTab[]>('list_browser_tabs');
      setTabs(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTabs();
  }, []);

  const handleTabClick = (index: number) => {
    if (selectedIndex === index) {
      setSelectedIndex(null);
      setGist(null);
      setGistError(null);
    } else {
      setSelectedIndex(index);
      setGist(null);
      setGistError(null);
    }
  };

  const handlePrepareGist = async () => {
    if (selectedIndex === null) return;
    const tab = tabs[selectedIndex];
    setGistLoading(true);
    setGistError(null);

    try {
      const result = await invoke<PageGist>('prepare_tab_gist', {
        url: tab.url,
        sourceType: tab.source_type,
      });
      console.log('[BrowserTool] Gist result:', JSON.stringify(result, null, 2));
      setGist(result);
    } catch (err) {
      setGistError(String(err));
    } finally {
      setGistLoading(false);
    }
  };

  const formatGist = (g: PageGist): string => {
    const lines = [`Gist of ${g.url}`, '', `Title: ${g.title}`];
    if (g.author) lines.push(`Author: ${g.author}`);
    if (g.published_date) lines.push(`Published: ${g.published_date}`);
    if (g.description) {
      lines.push('', 'Description:', g.description);
    }
    if (g.content_excerpt) {
      lines.push('', 'Article:', g.content_excerpt);
    }
    return lines.join('\n');
  };

  const handleCopy = () => {
    if (gist) {
      navigator.clipboard.writeText(formatGist(gist));
    }
  };

  const handleExport = async () => {
    console.log('[BrowserTool] Export clicked, gist:', !!gist);
    if (!gist) return;
    try {
      const content = formatGist(gist);
      console.log('[BrowserTool] Exporting, title:', gist.title, 'content length:', content.length);
      const path = await invoke<string>('export_gist', {
        title: gist.title,
        content,
      });
      console.log('[BrowserTool] Gist exported to:', path);
      setExportStatus(`Saved to: ${path}`);
    } catch (err) {
      console.error('[BrowserTool] Export failed:', err);
      setExportStatus(`Export failed: ${err}`);
    }
  };

  const handleDismiss = () => {
    setGist(null);
    setGistError(null);
    setExportStatus(null);
    setSelectedIndex(null);
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>Browser</h2>
        <button onClick={onClose} className="close-button">×</button>
      </div>
      <div className="settings-content">
        <div className="browser-toolbar">
          <button onClick={fetchTabs} className="refresh-button" disabled={loading}>
            {loading ? 'Loading...' : 'Refresh'}
          </button>
          <span className="tab-count">{tabs.length} tabs</span>
        </div>

        {error && (
          <div className="error-state">{error}</div>
        )}

        <div className="tab-list">
          {tabs.length === 0 && !loading && !error && (
            <p className="empty-state">No tabs found. Is Chrome running?</p>
          )}
          {tabs.map((tab, index) => (
            <div
              key={`${tab.url}-${index}`}
              className={`tab-item ${selectedIndex === index ? 'selected' : ''}`}
              onClick={() => handleTabClick(index)}
            >
              <div className="tab-item-content">
                <div className="tab-item-header">
                  <span className="tab-domain">{tab.domain}</span>
                  <span className={SOURCE_BADGES[tab.source_type].className}>
                    {SOURCE_BADGES[tab.source_type].label}
                  </span>
                </div>
                <div className="tab-title">{tab.title}</div>
              </div>
            </div>
          ))}
        </div>

        {selectedIndex !== null && !gist && (
          <div className="gist-action-bar">
            {!gistLoading && !gistError && (
              <button onClick={handlePrepareGist} className="prepare-gist-button">
                Prepare Gist
              </button>
            )}
            {gistLoading && (
              <div className="loading-state">Extracting gist...</div>
            )}
            {gistError && (
              <div className="error-state">Error: {gistError}</div>
            )}
          </div>
        )}

        {gist && (
          <GistCard
            gist={gist}
            onCopy={handleCopy}
            onExport={handleExport}
            onDismiss={handleDismiss}
          />
        )}

        {exportStatus && (
          <div className="loading-state" style={{ marginTop: '8px' }}>
            {exportStatus}
          </div>
        )}
      </div>
    </div>
  );
}
