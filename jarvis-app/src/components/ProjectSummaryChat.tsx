import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectSummaryResult } from '../state/types';

// Local types (not exported)
type SummaryState = 'empty' | 'generating' | 'review' | 'saved';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}

interface ProjectSummaryChatProps {
  projectId: string;
  projectTitle: string;
  onGemSaved?: () => void;
}

export default function ProjectSummaryChat({
  projectId,
  onGemSaved
}: ProjectSummaryChatProps) {
  const [state, setState] = useState<SummaryState>('empty');
  const [summaryResult, setSummaryResult] = useState<ProjectSummaryResult | null>(null);
  const [saved, setSaved] = useState(false);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initializing, setInitializing] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to latest message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages, loading]);

  // Load latest checkpoint on mount (or when project changes)
  useEffect(() => {
    let cancelled = false;

    const loadCheckpoint = async () => {
      setInitializing(true);
      try {
        const result = await invoke<ProjectSummaryResult | null>(
          'get_latest_project_summary_checkpoint',
          { projectId }
        );

        if (cancelled) return;

        if (result) {
          setSummaryResult(result);
          setState('review');
          setSaved(false);
        } else {
          setState('empty');
        }
      } catch (err) {
        if (cancelled) return;
        setState('empty');
      } finally {
        if (!cancelled) {
          setInitializing(false);
        }
      }

      // Reset Q&A state
      setChatMessages([]);
      setInput('');
      setError(null);
    };

    loadCheckpoint();

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  // Handler: Generate summary
  const handleGenerate = async () => {
    setState('generating');
    setError(null);
    try {
      const result = await invoke<ProjectSummaryResult>(
        'generate_project_summary_checkpoint',
        { projectId }
      );
      setSummaryResult(result);
      setState('review');
      setSaved(false);
      setChatMessages([]);
    } catch (err) {
      setState('empty');
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // Handler: Save summary as gem
  const handleSave = async () => {
    if (!summaryResult) return;
    try {
      await invoke('save_project_summary_checkpoint', {
        projectId,
        summaryContent: summaryResult.summary,
        compositeDoc: summaryResult.composite_doc,
      });
      setSaved(true);
      setState('saved');
      onGemSaved?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // Handler: Ask question about summary
  const handleAskQuestion = async () => {
    const question = input.trim();
    if (!question || loading || !summaryResult) return;

    setInput('');
    setChatMessages(prev => [...prev, { role: 'user', content: question }]);
    setLoading(true);

    try {
      const answer = await invoke<string>('send_summary_question', {
        question,
        summary: summaryResult.summary,
        compositeDoc: summaryResult.composite_doc,
      });
      setChatMessages(prev => [...prev, { role: 'assistant', content: answer }]);
    } catch (err) {
      setChatMessages(prev => [...prev, {
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : String(err)}`,
      }]);
    } finally {
      setLoading(false);
    }
  };

  // Handler: Enter key press
  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleAskQuestion();
    }
  };

  // Render: Loading state (while fetching checkpoint)
  if (initializing) {
    return (
      <div className="summary-chat-generating">
        <div className="spinner" />
        <span>Loading...</span>
      </div>
    );
  }

  // Render: Empty state
  if (state === 'empty') {
    return (
      <div className="summary-chat-empty">
        <h3>Project Summary</h3>
        <p>Generate a summary covering all key points from every gem in this project.</p>
        <button className="action-button" onClick={handleGenerate}>
          Generate Summary
        </button>
        {error && <p className="summary-error">{error}</p>}
      </div>
    );
  }

  // Render: Generating state
  if (state === 'generating') {
    return (
      <div className="summary-chat-generating">
        <div className="spinner" />
        <span>Analyzing gems...</span>
      </div>
    );
  }

  // Render: Review / Saved state
  return (
    <div className="summary-chat">
      <div className="summary-preview">
        <pre className="summary-content">{summaryResult?.summary}</pre>
        <div className="summary-meta">
          {summaryResult?.gems_analyzed} gems analyzed
          {summaryResult && summaryResult.chunks_used > 0 && ` · ${summaryResult.chunks_used} chunks`}
        </div>
      </div>

      <div className="summary-actions">
        {!saved ? (
          <button className="action-button" onClick={handleSave}>Save as Gem</button>
        ) : (
          <span className="summary-saved-badge">✓ Saved</span>
        )}
        <button className="action-button" onClick={handleGenerate}>Regenerate</button>
      </div>

      <div className="summary-qa">
        {chatMessages.map((msg, i) => (
          <div key={i} className={`chat-message chat-${msg.role}`}>
            <div className="chat-bubble">{msg.content}</div>
          </div>
        ))}
        {loading && (
          <div className="chat-message chat-assistant">
            <div className="chat-bubble thinking">Thinking...</div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-bar">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyPress}
          placeholder="Ask about the summary..."
          disabled={loading}
          className="chat-input"
        />
        <button
          onClick={handleAskQuestion}
          disabled={!input.trim() || loading}
          className="chat-send-button"
        >
          Send
        </button>
      </div>
      {error && <p className="summary-error">{error}</p>}
    </div>
  );
}
