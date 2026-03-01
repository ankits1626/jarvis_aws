import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectPreview, Project, ProjectDetail, GemPreview, GemSearchResult, Gem } from '../state/types';

interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
  onProjectSelect?: (id: string | null, title: string | null) => void;
  refreshTrigger?: number;
}

function CreateProjectForm({
  onCreated,
  onCancel,
}: {
  onCreated: (id: string, title: string) => void;
  onCancel: () => void;
}) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [objective, setObjective] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;

    setCreating(true);
    setError(null);

    try {
      const project = await invoke<Project>('create_project', {
        title: title.trim(),
        description: description.trim() || null,
        objective: objective.trim() || null,
      });
      onCreated(project.id, project.title);
    } catch (err) {
      setError(String(err));
      setCreating(false);
    }
  };

  return (
    <form className="create-project-form" onSubmit={handleSubmit}>
      <input
        type="text"
        placeholder="Project title (required)"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        autoFocus
        disabled={creating}
      />
      <textarea
        placeholder="Description (optional)"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        rows={2}
        disabled={creating}
      />
      <textarea
        placeholder="Objective (optional)"
        value={objective}
        onChange={(e) => setObjective(e.target.value)}
        rows={2}
        disabled={creating}
      />
      {error && (
        <div className="error-state">
          {error}
        </div>
      )}
      <div className="form-actions">
        <button
          type="button"
          className="action-button secondary"
          onClick={onCancel}
          disabled={creating}
        >
          Cancel
        </button>
        <button
          type="submit"
          className="action-button"
          disabled={!title.trim() || creating}
        >
          {creating ? 'Creating...' : 'Create Project'}
        </button>
      </div>
    </form>
  );
}

function AddGemsModal({
  projectId,
  projectTitle,
  existingGemIds,
  onClose,
  onAdded,
}: {
  projectId: string;
  projectTitle: string;
  existingGemIds: string[];
  onClose: () => void;
  onAdded: () => void;
}) {
  const [gems, setGems] = useState<GemSearchResult[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);

  // Load gems on mount
  useEffect(() => {
    loadGems('');
  }, []);

  // Debounced search (300ms)
  useEffect(() => {
    if (!searchQuery.trim()) return; // Skip debounce for initial empty load
    const timer = setTimeout(() => loadGems(searchQuery), 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const loadGems = async (query: string) => {
    setLoading(true);
    try {
      const results = await invoke<GemSearchResult[]>('search_gems', {
        query: query.trim(),
        limit: 100,
      });
      setGems(results);
    } catch (err) {
      console.error('Failed to load gems:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setSearchQuery(value);
    if (!value.trim()) {
      loadGems(''); // Immediately reload all gems when input cleared
    }
  };

  const toggleGem = (gemId: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(gemId)) next.delete(gemId);
      else next.add(gemId);
      return next;
    });
  };

  const handleAdd = async () => {
    setAdding(true);
    try {
      const gemIds = Array.from(selectedIds);
      await invoke('add_gems_to_project', { projectId, gemIds });
      onAdded();
    } catch (err) {
      console.error('Failed to add gems:', err);
      setAdding(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-card" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Add Gems to {projectTitle}</h3>
          <button className="close-button" onClick={onClose}>×</button>
        </div>
        <div className="modal-search">
          <input
            type="search"
            placeholder="Search gems..."
            value={searchQuery}
            onChange={handleSearchChange}
            className="gems-search-input"
            autoFocus
          />
        </div>
        <div className="modal-gem-list">
          {loading && gems.length === 0 && (
            <div className="empty-state">
              Loading gems...
            </div>
          )}
          {!loading && gems.length === 0 && (
            <div className="empty-state">
              {searchQuery.trim() ? 'No gems match your search.' : 'No gems available.'}
            </div>
          )}
          {gems.map(gem => {
            const alreadyAdded = existingGemIds.includes(gem.id);
            const isSelected = selectedIds.has(gem.id);
            return (
              <div
                key={gem.id}
                className={`modal-gem-row ${isSelected ? 'selected' : ''} ${alreadyAdded ? 'disabled' : ''}`}
                onClick={() => !alreadyAdded && toggleGem(gem.id)}
              >
                <input
                  type="checkbox"
                  checked={alreadyAdded || isSelected}
                  disabled={alreadyAdded}
                  readOnly
                />
                <div className="modal-gem-info">
                  <span className="modal-gem-title">{gem.title}</span>
                  <span className="modal-gem-meta">
                    {gem.source_type} · {gem.domain}
                  </span>
                </div>
                {alreadyAdded && (
                  <span className="already-added-label">
                    Already added
                  </span>
                )}
              </div>
            );
          })}
        </div>
        <div className="modal-footer">
          <button className="action-button secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            className="action-button"
            onClick={handleAdd}
            disabled={selectedIds.size === 0 || adding}
          >
            {adding ? 'Adding...' : `Add Selected (${selectedIds.size})`}
          </button>
        </div>
      </div>
    </div>
  );
}

function ProjectList({
  projects,
  selectedProjectId,
  onSelectProject,
  onProjectCreated,
  loading,
}: {
  projects: ProjectPreview[];
  selectedProjectId: string | null;
  onSelectProject: (id: string) => void;
  onProjectCreated: (id: string, title: string) => void;
  loading: boolean;
}) {
  const [showCreateForm, setShowCreateForm] = useState(false);

  return (
    <div className="project-list">
      <div className="project-list-header">
        <h3>Projects</h3>
        <button
          className="action-button"
          onClick={() => setShowCreateForm(true)}
        >
          + New Project
        </button>
      </div>

      {showCreateForm && (
        <CreateProjectForm
          onCreated={(id, title) => { setShowCreateForm(false); onProjectCreated(id, title); }}
          onCancel={() => setShowCreateForm(false)}
        />
      )}

      <div className="project-list-items">
        {loading && projects.length === 0 && (
          <div className="loading-state">
            Loading projects...
          </div>
        )}
        {!loading && projects.length === 0 && (
          <div className="empty-state">
            No projects yet. Click "+ New Project" to create one.
          </div>
        )}
        {projects.map(project => (
          <div
            key={project.id}
            className={`project-card ${selectedProjectId === project.id ? 'active' : ''}`}
            onClick={() => onSelectProject(project.id)}
          >
            <div className="project-card-title">{project.title}</div>
            <div className="project-card-meta">
              <span className={`status-badge status-${project.status}`}>
                {project.status}
              </span>
              <span className="gem-count">{project.gem_count} gems</span>
            </div>
            {project.description && (
              <div className="project-card-desc">{project.description}</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function ProjectGemCard({
  gem,
  onGemSelect,
  onRemove,
}: {
  gem: GemPreview;
  onGemSelect?: (gemId: string | null) => void;
  onRemove: (gemId: string) => void;
}) {
  const isAudioTranscript = gem.domain === 'jarvis-app';
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [audioLoading, setAudioLoading] = useState(false);

  const handlePlayToggle = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      setAudioUrl(null);
    } else {
      setAudioLoading(true);
      try {
        const fullGem = await invoke<Gem>('get_gem', { id: gem.id });
        const recordingFilename = fullGem.source_meta?.recording_filename as string | undefined;
        if (!recordingFilename) return;
        const wavBytes = await invoke<number[]>('convert_to_wav', { filename: recordingFilename });
        const blob = new Blob([new Uint8Array(wavBytes)], { type: 'audio/wav' });
        setAudioUrl(URL.createObjectURL(blob));
      } catch (err) {
        console.error('Failed to load audio:', err);
      } finally {
        setAudioLoading(false);
      }
    }
  };

  return (
    <div className="project-gem-card">
      <div
        className="gem-card"
        onClick={() => onGemSelect?.(gem.id)}
      >
        <div className="gem-card-header">
          <span className={`source-badge ${gem.source_type.toLowerCase()}`}>
            {gem.source_type}
          </span>
          {isAudioTranscript && gem.transcript_language && (
            <span className="gem-lang-badge" title="Transcript available">
              {gem.transcript_language}
            </span>
          )}
          <span className="gem-date">
            {new Date(gem.captured_at).toLocaleDateString()}
          </span>
        </div>
        <div className="gem-title">{gem.title}</div>
        {gem.author && (
          <div className="gem-meta">
            <span className="gem-author">by {gem.author}</span>
          </div>
        )}
        {gem.description && (
          <div className="gem-description">{gem.description}</div>
        )}
        {gem.tags && gem.tags.length > 0 && (
          <div className="gem-tags">
            {gem.tags.map((tag, idx) => (
              <span key={idx} className="gem-tag">{tag}</span>
            ))}
          </div>
        )}
        {isAudioTranscript && (
          <div className="gem-card-audio-controls">
            <button
              className="gem-play-button"
              onClick={handlePlayToggle}
              disabled={audioLoading}
            >
              {audioLoading ? '...' : audioUrl ? 'Stop' : 'Play'}
            </button>
          </div>
        )}
        {audioUrl && (
          <div className="gem-audio-player" onClick={(e) => e.stopPropagation()}>
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
      </div>
      <button
        className="remove-from-project"
        onClick={(e) => { e.stopPropagation(); onRemove(gem.id); }}
        title="Remove from project"
      >
        ×
      </button>
    </div>
  );
}

function ProjectGemList({
  projectId,
  onGemSelect,
  onProjectsChanged,
  onProjectDeleted,
  refreshTrigger,
}: {
  projectId: string | null;
  onGemSelect?: (gemId: string | null) => void;
  onProjectsChanged: () => void;
  onProjectDeleted: () => void;
  refreshTrigger?: number;
}) {
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<GemPreview[] | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [editObjective, setEditObjective] = useState('');
  const [editStatus, setEditStatus] = useState('');
  const [saving, setSaving] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);

  useEffect(() => {
    if (!projectId) {
      setDetail(null);
      setSearchQuery('');
      setSearchResults(null);
      return;
    }
    loadProject(projectId);
  }, [projectId, refreshTrigger]);

  // Search with 300ms debounce
  useEffect(() => {
    if (!projectId || !searchQuery.trim()) {
      setSearchResults(null);
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const results = await invoke<GemPreview[]>('get_project_gems', {
          projectId,
          query: searchQuery.trim(),
          limit: 100,
        });
        setSearchResults(results);
      } catch (err) {
        console.error('Failed to search project gems:', err);
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, projectId]);

  const loadProject = async (id: string) => {
    setLoading(true);
    try {
      const result = await invoke<ProjectDetail>('get_project', { id });
      setDetail(result);
    } catch (err) {
      console.error('Failed to load project:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleRemoveGem = async (gemId: string) => {
    if (!projectId) return;
    try {
      await invoke('remove_gem_from_project', { projectId, gemId });
      loadProject(projectId);
      onProjectsChanged();
    } catch (err) {
      console.error('Failed to remove gem:', err);
    }
  };

  const handleDeleteProject = async () => {
    if (!projectId) return;
    try {
      await invoke('delete_project', { id: projectId });
      setConfirmDelete(false);
      onProjectDeleted();
    } catch (err) {
      console.error('Failed to delete project:', err);
    }
  };

  const startEditing = () => {
    if (!detail) return;
    setEditTitle(detail.project.title);
    setEditDescription(detail.project.description || '');
    setEditObjective(detail.project.objective || '');
    setEditStatus(detail.project.status);
    setEditError(null);
    setEditing(true);
  };

  const cancelEditing = () => {
    setEditing(false);
    setEditError(null);
  };

  const handleSave = async () => {
    if (!projectId || !editTitle.trim()) return;
    setSaving(true);
    setEditError(null);
    try {
      await invoke<Project>('update_project', {
        id: projectId,
        title: editTitle.trim(),
        description: editDescription.trim() || null,
        objective: editObjective.trim() || null,
        status: editStatus,
      });
      setEditing(false);
      loadProject(projectId);
      onProjectsChanged(); // refresh project list (title/status may have changed)
    } catch (err) {
      setEditError(String(err));
    } finally {
      setSaving(false);
    }
  };

  if (!projectId) {
    return (
      <div className="project-gem-list empty-state">
        Select a project to see its gems
      </div>
    );
  }

  if (loading && !detail) {
    return (
      <div className="project-gem-list loading">
        Loading...
      </div>
    );
  }

  if (!detail) return null;

  const displayGems = searchResults !== null ? searchResults : detail.gems;

  return (
    <div className="project-gem-list">
      {/* Project metadata header */}
      <div className="project-metadata-header">
        {editing ? (
          /* Edit mode */
          <div className="project-edit-form">
            <input
              type="text"
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              placeholder="Project title (required)"
              autoFocus
              disabled={saving}
            />
            <div className="edit-status-row">
              <label>Status:</label>
              <select
                value={editStatus}
                onChange={(e) => setEditStatus(e.target.value)}
                disabled={saving}
              >
                <option value="active">Active</option>
                <option value="paused">Paused</option>
                <option value="completed">Completed</option>
                <option value="archived">Archived</option>
              </select>
            </div>
            <textarea
              value={editDescription}
              onChange={(e) => setEditDescription(e.target.value)}
              placeholder="Description (optional)"
              rows={2}
              disabled={saving}
            />
            <textarea
              value={editObjective}
              onChange={(e) => setEditObjective(e.target.value)}
              placeholder="Objective (optional)"
              rows={2}
              disabled={saving}
            />
            {editError && (
              <div className="error-state">
                {editError}
              </div>
            )}
            <div className="edit-actions">
              <button
                className="action-button secondary"
                onClick={cancelEditing}
                disabled={saving}
              >
                Cancel
              </button>
              <button
                className="action-button"
                onClick={handleSave}
                disabled={!editTitle.trim() || saving}
              >
                {saving ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>
        ) : (
          /* Display mode */
          <>
            <h2>{detail.project.title}</h2>
            <div className="project-meta-row">
              <span className={`status-badge status-${detail.project.status}`}>
                {detail.project.status}
              </span>
              <span>{detail.gem_count} gems</span>
              <button className="action-button small" onClick={startEditing}>
                Edit
              </button>
              <button className="action-button small danger" onClick={() => setConfirmDelete(true)}>
                Delete
              </button>
            </div>
            {detail.project.objective && (
              <div className="project-objective">{detail.project.objective}</div>
            )}
            {detail.project.description && (
              <div className="project-description">
                {detail.project.description}
              </div>
            )}
          </>
        )}
      </div>

      {/* Delete confirmation */}
      {confirmDelete && (
        <div className="delete-confirm-bar">
          <span>Delete "{detail.project.title}"? This cannot be undone. Gems will not be deleted.</span>
          <button className="action-button small danger" onClick={handleDeleteProject}>
            Confirm Delete
          </button>
          <button className="action-button small" onClick={() => setConfirmDelete(false)}>
            Cancel
          </button>
        </div>
      )}

      {/* Toolbar */}
      <div className="project-gem-toolbar">
        <input
          type="search"
          placeholder="Search project gems..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="gems-search-input"
        />
        <button
          className="action-button"
          onClick={() => setShowAddModal(true)}
        >
          + Add Gems
        </button>
      </div>

      {/* Gem cards */}
      <div className="project-gems-list">
        {displayGems.length === 0 && (
          <div className="empty-state">
            {searchQuery.trim()
              ? 'No gems match your search.'
              : 'No gems in this project. Click "+ Add Gems" to get started.'}
          </div>
        )}
        {displayGems.map(gem => (
          <ProjectGemCard
            key={gem.id}
            gem={gem}
            onGemSelect={onGemSelect}
            onRemove={handleRemoveGem}
          />
        ))}
      </div>

      {/* Add Gems Modal */}
      {showAddModal && (
        <AddGemsModal
          projectId={projectId}
          projectTitle={detail.project.title}
          existingGemIds={detail.gems.map(g => g.id)}
          onClose={() => setShowAddModal(false)}
          onAdded={() => {
            setShowAddModal(false);
            loadProject(projectId);
            onProjectsChanged();
          }}
        />
      )}
    </div>
  );
}

export function ProjectsContainer({ onGemSelect, onProjectSelect, refreshTrigger }: ProjectsContainerProps) {
  const [projects, setProjects] = useState<ProjectPreview[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchProjects = useCallback(async () => {
    try {
      const result = await invoke<ProjectPreview[]>('list_projects');
      setProjects(result);
    } catch (err) {
      console.error('Failed to load projects:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchProjects(); }, [fetchProjects]);

  const handleProjectCreated = (projectId: string, projectTitle: string) => {
    fetchProjects();
    setSelectedProjectId(projectId);
    onProjectSelect?.(projectId, projectTitle);
  };

  return (
    <div className="projects-container">
      <ProjectList
        projects={projects}
        selectedProjectId={selectedProjectId}
        onSelectProject={(id) => {
          setSelectedProjectId(id);
          const project = projects.find(p => p.id === id);
          onProjectSelect?.(id, project?.title || null);
        }}
        onProjectCreated={handleProjectCreated}
        loading={loading}
      />
      <ProjectGemList
        projectId={selectedProjectId}
        onGemSelect={onGemSelect}
        onProjectsChanged={fetchProjects}
        onProjectDeleted={() => {
          setSelectedProjectId(null);
          onProjectSelect?.(null, null);
          fetchProjects();
        }}
        refreshTrigger={refreshTrigger}
      />
    </div>
  );
}
