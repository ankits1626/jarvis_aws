import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { BrowserTab, PageGist, SourceType, Gem, AvailabilityResult, ClaudePanelStatus, ProjectPreview } from '../state/types';

interface BrowserToolProps {
  onClose?: () => void;
}

/** Compare two URLs by page identity, ignoring volatile query params like t=, si=, feature=.
 *  For YouTube, matches on origin + pathname + video ID (v=).
 *  For other sites, matches on origin + pathname. */
function urlsSamePage(a: string, b: string): boolean {
  try {
    const ua = new URL(a);
    const ub = new URL(b);
    if (ua.origin !== ub.origin || ua.pathname !== ub.pathname) return false;
    // For YouTube watch pages, the video ID (v=) is the page identity
    if (ua.hostname.includes('youtube.com') && ua.pathname === '/watch') {
      return ua.searchParams.get('v') === ub.searchParams.get('v');
    }
    return true;
  } catch {
    return a === b;
  }
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

function GistCard({ gist, onCopy, onDismiss }: { gist: PageGist; onCopy: () => void; onDismiss: () => void }) {
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [aiAvailability, setAiAvailability] = useState<AvailabilityResult | null>(null);
  const [savedGemId, setSavedGemId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectPreview[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string>('');
  const [addingToProject, setAddingToProject] = useState(false);
  const [addedToProject, setAddedToProject] = useState<string | null>(null);

  // Check AI availability on mount
  useEffect(() => {
    const checkAvailability = async () => {
      try {
        const result = await invoke<AvailabilityResult>('check_intel_availability');
        setAiAvailability(result);
      } catch (err) {
        console.error('Failed to check AI availability:', err);
      }
    };
    checkAvailability();
  }, []);

  const durationSeconds = gist.extra?.duration_seconds as number | undefined;
  const formatDuration = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveError(null);

    try {
      const gem = await invoke<Gem>('save_gem', { gist });
      setSaved(true);
      setSavedGemId(gem.id);

      // Fetch projects for the "Add to Project" picker
      try {
        const projectList = await invoke<ProjectPreview[]>('list_projects');
        setProjects(projectList.filter(p => p.status === 'active'));
      } catch {
        // Non-fatal: project picker just won't appear
      }
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleAddToProject = async () => {
    if (!savedGemId || !selectedProjectId) return;
    setAddingToProject(true);
    try {
      await invoke('add_gems_to_project', { projectId: selectedProjectId, gemIds: [savedGemId] });
      const project = projects.find(p => p.id === selectedProjectId);
      setAddedToProject(project?.title || 'project');
    } catch (err) {
      console.error('Failed to add gem to project:', err);
    } finally {
      setAddingToProject(false);
    }
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
        <button
          onClick={handleSave}
          className="save-gem-button"
          disabled={saved || saving}
        >
          {saved ? 'Saved' : saving ? 'Saving...' : 'Save Gem'}
        </button>
        <button onClick={onDismiss} className="gist-dismiss-button">Dismiss</button>
      </div>
      {aiAvailability?.available && !saved && (
        <div className="ai-enrichment-notice">
          ✨ AI enrichment will be added on save
        </div>
      )}
      {saveError && (
        <div className="error-state" style={{ marginTop: '8px' }}>
          {saveError}
        </div>
      )}
      {saved && projects.length > 0 && !addedToProject && (
        <div className="gist-add-to-project">
          <select
            value={selectedProjectId}
            onChange={(e) => setSelectedProjectId(e.target.value)}
            className="project-select"
          >
            <option value="">Add to project...</option>
            {projects.map(p => (
              <option key={p.id} value={p.id}>{p.title}</option>
            ))}
          </select>
          <button
            onClick={handleAddToProject}
            disabled={!selectedProjectId || addingToProject}
            className="add-to-project-button"
          >
            {addingToProject ? 'Adding...' : 'Add'}
          </button>
        </div>
      )}
      {addedToProject && (
        <div className="added-to-project-notice">
          Added to {addedToProject}
        </div>
      )}
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
  
  // Claude panel detection state
  const [claudeStatus, setClaudeStatus] = useState<ClaudePanelStatus | null>(null);

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

  const checkClaudePanel = async () => {
    try {
      const status = await invoke<ClaudePanelStatus>('check_claude_panel');
      setClaudeStatus(status);
    } catch {
      setClaudeStatus(null);
    }
  };

  // Poll for Claude panel status every 3 seconds
  useEffect(() => {
    checkClaudePanel();
    const interval = setInterval(checkClaudePanel, 3000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    fetchTabs();
  }, []);

  const isClaudeOnTab = (tab: BrowserTab): boolean => {
    if (!claudeStatus?.detected || !claudeStatus.active_tab_url) return false;
    // Compare page identity, not exact URL — YouTube and other sites
    // dynamically update query params (t=, si=, feature=, etc.)
    return urlsSamePage(tab.url, claudeStatus.active_tab_url);
  };

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
      const command = isClaudeOnTab(tab)
        ? 'prepare_tab_gist_with_claude'
        : 'prepare_tab_gist';
      const result = await invoke<PageGist>(command, {
        url: tab.url,
        sourceType: tab.source_type,
      });
      console.log(`[BrowserTool] Gist result (${command}):`, JSON.stringify(result, null, 2));
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

  const handleDismiss = () => {
    setGist(null);
    setGistError(null);
    setSelectedIndex(null);
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>Browser</h2>
        {onClose && <button onClick={onClose} className="close-button">×</button>}
      </div>
      <div className="settings-content">
        <div className="browser-toolbar">
          <button onClick={fetchTabs} className="refresh-button" disabled={loading}>
            {loading ? 'Loading...' : 'Refresh'}
          </button>
          <span className="tab-count">{tabs.length} tabs</span>
        </div>

        {claudeStatus?.needs_accessibility && (
          <div className="accessibility-notice">
            Claude extension detection requires Accessibility permission.
            <br />
            System Settings → Privacy & Security → Accessibility → Enable JarvisApp
          </div>
        )}

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
                  <div className="tab-badges">
                    {isClaudeOnTab(tab) && (
                      <span className="claude-badge" title="Claude conversation will be included">Claude</span>
                    )}
                    <span className={SOURCE_BADGES[tab.source_type].className}>
                      {SOURCE_BADGES[tab.source_type].label}
                    </span>
                  </div>
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
                {selectedIndex !== null && isClaudeOnTab(tabs[selectedIndex])
                  ? 'Prepare Gist + Claude'
                  : 'Prepare Gist'}
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
            onDismiss={handleDismiss}
          />
        )}
      </div>
    </div>
  );
}
