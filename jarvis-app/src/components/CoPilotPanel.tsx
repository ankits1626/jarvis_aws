import { useState } from 'react';
import type { CoPilotState, CoPilotStatus } from '../state/types';

interface CoPilotPanelProps {
  state: CoPilotState | null;
  status: CoPilotStatus;
  error: string | null;
  onDismissQuestion: (index: number) => void;
}

export function CoPilotPanel({ state, status, error, onDismissQuestion }: CoPilotPanelProps) {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  // Show placeholder when no state yet
  if (!state || state.cycle_metadata.cycle_number === 0) {
    return (
      <div className="copilot-panel">
        <div className="copilot-placeholder">
          <p>Co-Pilot is analyzing your conversation...</p>
          <p className="copilot-placeholder-hint">
            Insights will appear here as the conversation progresses.
          </p>
        </div>
      </div>
    );
  }

  const handleQuestionClick = (question: string, index: number) => {
    navigator.clipboard.writeText(question);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  const formatTimeAgo = (timestamp: string): string => {
    const now = new Date();
    const then = new Date(timestamp);
    const seconds = Math.floor((now.getTime() - then.getTime()) / 1000);

    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  };

  return (
    <div className="copilot-panel">
      {/* Error display */}
      {error && (
        <div className="copilot-error">
          <span className="error-icon">⚠️</span>
          <span>{error}</span>
        </div>
      )}

      {/* Summary Section */}
      {state.running_summary && (
        <div className="copilot-section">
          <h3>Summary</h3>
          <p className="copilot-summary">{state.running_summary}</p>
        </div>
      )}

      {/* Key Points */}
      {state.key_points.length > 0 && (
        <div className="copilot-section">
          <h4>Key Points</h4>
          <ul className="copilot-key-points">
            {state.key_points.map((point, i) => (
              <li key={i}>{point}</li>
            ))}
          </ul>
        </div>
      )}

      {/* Open Questions */}
      {state.open_questions.length > 0 && (
        <div className="copilot-section">
          <h4>Open Questions</h4>
          <ul className="copilot-open-questions">
            {state.open_questions.map((question, i) => (
              <li key={i} className="open-question">{question}</li>
            ))}
          </ul>
        </div>
      )}

      {/* Decisions & Action Items */}
      {(state.decisions.length > 0 || state.action_items.length > 0) && (
        <div className="copilot-section">
          <h4>Decisions & Action Items</h4>
          
          {state.decisions.length > 0 && (
            <div className="copilot-decisions">
              <h5>Decisions</h5>
              <ul>
                {state.decisions.map((decision, i) => (
                  <li key={i} className="decision-item">
                    <span className="decision-icon">✓</span>
                    {decision}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {state.action_items.length > 0 && (
            <div className="copilot-action-items">
              <h5>Action Items</h5>
              <ul>
                {state.action_items.map((item, i) => (
                  <li key={i}>{item}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Suggested Questions */}
      {state.suggested_questions.filter(q => !q.dismissed).length > 0 && (
        <div className="copilot-section">
          <h4>Suggested Questions</h4>
          <div className="copilot-questions-grid">
            {state.suggested_questions
              .map((q, originalIndex) => ({ ...q, originalIndex }))
              .filter(q => !q.dismissed)
              .map((q) => (
                <div
                  key={q.originalIndex}
                  className="copilot-question-card"
                  onClick={() => handleQuestionClick(q.question, q.originalIndex)}
                >
                  <div className="question-content">
                    <p className="question-text">{q.question}</p>
                    <p className="question-reason">{q.reason}</p>
                  </div>
                  <button
                    className="dismiss-button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDismissQuestion(q.originalIndex);
                    }}
                    aria-label="Dismiss question"
                  >
                    ×
                  </button>
                  {copiedIndex === q.originalIndex && (
                    <div className="copied-indicator">Copied!</div>
                  )}
                </div>
              ))}
          </div>
        </div>
      )}

      {/* Key Concepts */}
      {state.key_concepts.length > 0 && (
        <div className="copilot-section">
          <h4>Key Concepts</h4>
          <div className="copilot-concepts-grid">
            {state.key_concepts.map((concept, i) => (
              <div
                key={i}
                className="copilot-concept-chip"
                title={concept.context}
              >
                <span className="concept-term">{concept.term}</span>
                {concept.mention_count > 1 && (
                  <span className="mention-count">{concept.mention_count}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Status Footer */}
      <div className="copilot-footer">
        <div className="copilot-status">
          <span className={`status-indicator status-${status}`}>
            {status === 'processing' && <span className="pulse-dot" />}
          </span>
          <span className="status-text">
            Cycle {state.cycle_metadata.cycle_number}
            {' · '}
            {formatTimeAgo(state.cycle_metadata.last_updated_at)}
          </span>
        </div>
        {state.cycle_metadata.failed_cycles > 0 && (
          <div className="failed-cycles-warning">
            {state.cycle_metadata.failed_cycles} cycle(s) failed
          </div>
        )}
      </div>
    </div>
  );
}
