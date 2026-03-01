import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { ProjectResearchResults } from '../state/types';

// Local ChatMessage interface (NOT from types.ts)
interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  researchResults?: ProjectResearchResults;
  suggestedTopics?: string[];
}

interface ResearchChatState {
  messages: ChatMessage[];
  topics: string[];
  addedGemIds: string[];
  savedAt: string;
}

interface ProjectResearchChatProps {
  projectId: string;
  projectTitle: string;
  onGemsAdded?: () => void;
}

export default function ProjectResearchChat({
  projectId,
  projectTitle,
  onGemsAdded
}: ProjectResearchChatProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [topics, setTopics] = useState<string[]>([]);
  const [addedGemIds, setAddedGemIds] = useState<Set<string>>(new Set());
  const [initializing, setInitializing] = useState(true);
  const [stateLoaded, setStateLoaded] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to latest message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  // Load saved state or initialize fresh
  useEffect(() => {
    let cancelled = false;
    const initialize = async () => {
      setInitializing(true);
      try {
        // Try to restore saved state
        const savedJson = await invoke<string | null>('load_project_research_state', { projectId });
        if (cancelled) return;

        if (savedJson) {
          const saved: ResearchChatState = JSON.parse(savedJson);
          setMessages(saved.messages);
          setTopics(saved.topics);
          setAddedGemIds(new Set(saved.addedGemIds));
          setStateLoaded(true);
          setInitializing(false);
          return;
        }

        // No saved state — suggest topics fresh
        const suggested = await invoke<string[]>('suggest_project_topics', { projectId });
        if (cancelled) return;
        setTopics(suggested);
        setMessages([{
          role: 'assistant',
          content: `I see your project is about **${projectTitle}**. Here are some research topics I'd suggest:`,
          suggestedTopics: suggested,
        }]);
      } catch (err) {
        if (cancelled) return;
        setMessages([{
          role: 'assistant',
          content: `I couldn't initialize: ${err}. You can type your own research topics below.`,
        }]);
      } finally {
        if (!cancelled) {
          setInitializing(false);
          setStateLoaded(true);
        }
      }
    };
    initialize();
    return () => { cancelled = true; };
  }, [projectId, projectTitle]);

  // Auto-save state with 1s debounce
  useEffect(() => {
    if (!stateLoaded) return;
    const timer = setTimeout(async () => {
      const state: ResearchChatState = {
        messages,
        topics,
        addedGemIds: Array.from(addedGemIds),
        savedAt: new Date().toISOString(),
      };
      try {
        await invoke('save_project_research_state', {
          projectId,
          state: JSON.stringify(state),
        });
      } catch (err) {
        console.error('Failed to save research state:', err);
      }
    }, 1000);
    return () => clearTimeout(timer);
  }, [messages, topics, addedGemIds, stateLoaded, projectId]);

  // Execute research with curated topics
  const handleRunResearch = useCallback(async (researchTopics: string[]) => {
    if (researchTopics.length === 0) return;

    setLoading(true);
    setMessages(prev => [
      ...prev,
      { role: 'user', content: `Search for: ${researchTopics.join(', ')}` },
      { role: 'assistant', content: `Searching ${researchTopics.length} topics...` },
    ]);

    try {
      const results = await invoke<ProjectResearchResults>('run_project_research', {
        projectId,
        topics: researchTopics,
      });

      // Replace the "Searching..." placeholder with actual results
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = {
          role: 'assistant',
          content: `Found ${results.web_results.length} web resources and ${results.suggested_gems.length} matching gems from your library.`,
          researchResults: results,
        };
        return updated;
      });
    } catch (err) {
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = {
          role: 'assistant',
          content: `Research failed: ${err}. You can try again or refine your topics.`,
        };
        return updated;
      });
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  // Keyword-based intent detection
  const handleSendMessage = async () => {
    if (!input.trim() || loading) return;
    const userMessage = input.trim();
    setInput('');

    const lower = userMessage.toLowerCase();

    // Intent: Run research on curated topics
    if (lower.includes('search') || lower.includes('go ahead') || lower.includes('find')) {
      await handleRunResearch(topics);
      return;
    }

    // Intent: Summarize project
    if (lower.includes('summarize') || lower.includes('summary')) {
      setLoading(true);
      setMessages(prev => [
        ...prev,
        { role: 'user', content: userMessage },
        { role: 'assistant', content: 'Summarizing your project...' },
      ]);

      try {
        const summary = await invoke<string>('get_project_summary', { projectId });
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = { role: 'assistant', content: summary };
          return updated;
        });
      } catch (err) {
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = { role: 'assistant', content: `Failed to summarize: ${err}` };
          return updated;
        });
      } finally {
        setLoading(false);
      }
      return;
    }

    // Default: treat as a new topic to add to curated list
    setTopics(prev => [...prev, userMessage]);
    setMessages(prev => [
      ...prev,
      { role: 'user', content: userMessage },
      {
        role: 'assistant',
        content: `Added "${userMessage}" to your research topics. Say "search" when you're ready.`,
      },
    ]);
  };

  // Remove a topic from the curated list
  const handleRemoveTopic = (index: number) => {
    const removed = topics[index];
    setTopics(prev => prev.filter((_, i) => i !== index));
    setMessages(prev => [
      ...prev,
      { role: 'system', content: `Removed topic: "${removed}"` },
    ]);
  };

  // Add a gem suggestion to the project
  const handleAddGem = async (gemId: string) => {
    try {
      await invoke('add_gems_to_project', { projectId, gemIds: [gemId] });
      setAddedGemIds(prev => new Set(prev).add(gemId));
      onGemsAdded?.();
    } catch (err) {
      console.error('Failed to add gem:', err);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  // Clear saved state and re-initialize
  const handleNewResearch = useCallback(async () => {
    try {
      await invoke('clear_project_research_state', { projectId });
    } catch (err) {
      console.error('Failed to clear research state:', err);
    }
    setMessages([]);
    setTopics([]);
    setAddedGemIds(new Set());
    setStateLoaded(false);
    setInitializing(true);

    try {
      const suggested = await invoke<string[]>('suggest_project_topics', { projectId });
      setTopics(suggested);
      setMessages([{
        role: 'assistant',
        content: `I see your project is about **${projectTitle}**. Here are some research topics I'd suggest:`,
        suggestedTopics: suggested,
      }]);
    } catch (err) {
      setMessages([{
        role: 'assistant',
        content: `I couldn't generate topics: ${err}. You can type your own research topics below.`,
      }]);
    } finally {
      setInitializing(false);
      setStateLoaded(true);
    }
  }, [projectId, projectTitle]);

  // Initializing state
  if (initializing) {
    return (
      <div className="research-chat">
        <div className="research-chat-loading">
          <div className="spinner" />
          <span>Analyzing your project...</span>
        </div>
      </div>
    );
  }

  // Main chat UI
  return (
    <div className="research-chat">
      <div className="research-chat-messages">
        {messages.map((msg, index) => (
          <div key={index} className={`chat-message chat-${msg.role}`}>
            {msg.role !== 'system' ? (
              <div className="chat-bubble">
                <div className="chat-text">{msg.content}</div>

                {/* Topic chips with remove buttons */}
                {msg.suggestedTopics && (
                  <div className="research-topics-list">
                    {topics.map((topic, i) => (
                      <div key={i} className="research-topic-chip">
                        <span>{i + 1}. {topic}</span>
                        <button className="topic-remove" onClick={() => handleRemoveTopic(i)}>×</button>
                      </div>
                    ))}
                    <button
                      className="action-button research-go-button"
                      onClick={() => handleRunResearch(topics)}
                      disabled={topics.length === 0 || loading}
                    >
                      Search ({topics.length} topics)
                    </button>
                  </div>
                )}

                {/* Web result cards (no Add button - web results can't be added directly) */}
                {msg.researchResults && msg.researchResults.web_results.length > 0 && (
                  <div className="research-section">
                    <h4 className="research-section-title">From the web</h4>
                    {msg.researchResults.web_results.map((result, i) => (
                      <div key={i} className="web-result-card" onClick={() => open(result.url)}>
                        <div className="web-result-header">
                          <span className={`source-type-badge source-${result.source_type.toLowerCase()}`}>
                            {result.source_type}
                          </span>
                          <span className="web-result-domain">{result.domain}</span>
                        </div>
                        <div className="web-result-title">{result.title}</div>
                        <div className="web-result-snippet">{result.snippet}</div>
                      </div>
                    ))}
                  </div>
                )}

                {/* Gem suggestion cards (with Add button - these are existing gems) */}
                {msg.researchResults && msg.researchResults.suggested_gems.length > 0 && (
                  <div className="research-section">
                    <h4 className="research-section-title">From your library</h4>
                    {msg.researchResults.suggested_gems.map((gem) => (
                      <div key={gem.id} className="research-gem-card">
                        <div className="gem-info">
                          <span className={`source-badge ${gem.source_type.toLowerCase()}`}>{gem.source_type}</span>
                          <span className="gem-title">{gem.title}</span>
                        </div>
                        <button
                          className={`research-add-gem ${addedGemIds.has(gem.id) ? 'added' : ''}`}
                          onClick={() => handleAddGem(gem.id)}
                          disabled={addedGemIds.has(gem.id)}
                        >
                          {addedGemIds.has(gem.id) ? 'Added' : '+ Add'}
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="chat-system-msg">{msg.content}</div>
            )}
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
          onKeyDown={handleKeyDown}
          placeholder="Add a topic, say 'search', or ask a question..."
          disabled={loading}
          className="chat-input"
        />
        <button
          onClick={handleSendMessage}
          disabled={!input.trim() || loading}
          className="chat-send-button"
        >
          Send
        </button>
        <button
          className="research-new-button"
          onClick={handleNewResearch}
          disabled={loading || initializing}
          title="Clear research and start fresh"
        >
          New
        </button>
      </div>
    </div>
  );
}
