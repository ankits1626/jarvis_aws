/**
 * CoPilotCardStack Component
 * 
 * Animated card-based interface for Co-Pilot agent live intelligence display.
 * Transforms static document-style panel into individual cards that animate in,
 * auto-collapse after timeout, and persist after recording stops with summary card.
 */

import { useState, useRef, useEffect, useMemo } from 'react';
import {
  CoPilotState,
  CoPilotStatus,
  CoPilotCard,
  FinalSummaryCard,
  RecordingState,
  CoPilotRunningStatus,
} from '../state/types';
import {
  createCardsFromStateDiff,
  createFinalSummaryCard,
  AutoCollapseTimerManager,
  formatTimestamp,
  CARD_TYPE_LABEL,
} from '../utils/cardUtils';

/**
 * Props interface for CoPilotCardStack component
 */
interface CoPilotCardStackProps {
  /** Current Co-Pilot state from Tauri events */
  state: CoPilotState | null;
  
  /** Agent status (idle, processing, stopped, error) */
  status: CoPilotStatus;
  
  /** Error message if agent fails */
  error: string | null;
  
  /** Recording state from app state (idle, recording, processing) */
  recordingState: RecordingState;
  
  /** Seconds between cycles (from settings, default 60) */
  cycleInterval: number;
  
  /** Callback for dismissing suggested questions (not used in v1) */
  onDismissQuestion: (index: number) => void;
}

/**
 * CoPilotCardStack Component
 * 
 * Main component that manages card state, animations, and user interactions.
 */
export function CoPilotCardStack({
  state,
  status,
  error: _error, // Reserved for future error handling
  recordingState,
  cycleInterval,
  onDismissQuestion: _onDismissQuestion, // Reserved for future question dismissal
}: CoPilotCardStackProps) {
  // Component state
  const [cards, setCards] = useState<CoPilotCard[]>([]);
  const [finalSummaryCard, setFinalSummaryCard] = useState<FinalSummaryCard | null>(null);
  const [hasCompleted, setHasCompleted] = useState(false);
  const [nextCycleIn, setNextCycleIn] = useState(cycleInterval);

  // Refs for tracking previous values and DOM elements
  const previousState = useRef<CoPilotState | null>(null);
  const previousRecordingState = useRef<RecordingState>('idle');
  const previousCycleNumber = useRef(0);
  const cardAreaRef = useRef<HTMLDivElement>(null);

  // Timer manager for auto-collapse behavior
  const timerManager = useRef(
    new AutoCollapseTimerManager((cardId: string) => {
      // Callback when timer expires - collapse the card
      setCards(prevCards =>
        prevCards.map(card =>
          card.id === cardId
            ? { ...card, isExpanded: false, isNew: false }
            : card
        )
      );
    })
  );

  // Derive runningStatus from hasCompleted flag and props
  const runningStatus: CoPilotRunningStatus = useMemo(() => {
    if (hasCompleted) return 'complete';
    if (status === 'processing') return 'processing';
    if (recordingState === 'recording') return 'recording';
    return 'idle';
  }, [hasCompleted, recordingState, status]);

  // Task 6: Card Creation useEffect
  // Watch for CoPilotState changes and create new cards
  useEffect(() => {
    if (!state || state.cycle_metadata.cycle_number === 0) return;

    // Use functional setState to access latest cards without adding to deps
    setCards(prevCards => {
      const newCards = createCardsFromStateDiff(state, previousState.current, prevCards);
      
      if (newCards.length > 0) {
        // Start auto-collapse timers for new cards
        newCards.forEach(card => {
          const delay = card.type === 'summary_update' ? 5 : 8;
          timerManager.current.startTimer(card.id, delay);
        });

        // Scroll to top to show new cards
        cardAreaRef.current?.scrollTo({ top: 0, behavior: 'smooth' });

        return [...newCards, ...prevCards];
      }

      return prevCards;
    });

    previousState.current = state;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state]); // Only depend on state - cards accessed via functional setState

  // Tasks 7 & 8: Recording State Tracking + Final Summary Card Creation (Combined)
  // Detect recording state transitions to set hasCompleted flag and create final summary card
  // CRITICAL: These must be in the same useEffect to avoid ref mutation race condition
  useEffect(() => {
    const wasRecording = previousRecordingState.current === 'recording';
    const nowStopped = recordingState === 'idle';

    if (wasRecording && nowStopped) {
      setHasCompleted(true);

      // Create final summary card when recording stops
      if (state && state.cycle_metadata.cycle_number > 0) {
        setFinalSummaryCard(createFinalSummaryCard(state));
      }
    }

    if (recordingState === 'recording') {
      setHasCompleted(false);
    }

    previousRecordingState.current = recordingState;
  }, [recordingState, state]);

  // Task 9: Countdown Timer useEffect
  // Manage countdown timer for footer "Next cycle in ~Xs"
  useEffect(() => {
    if (!state) return;

    const currentCycleNumber = state.cycle_metadata.cycle_number;

    // Reset countdown when cycle number changes (new cycle started)
    if (currentCycleNumber !== previousCycleNumber.current) {
      setNextCycleIn(cycleInterval);
      previousCycleNumber.current = currentCycleNumber;
    }

    // Decrement countdown every second during recording
    if (recordingState === 'recording' && status !== 'processing') {
      const interval = setInterval(() => {
        setNextCycleIn(prev => Math.max(0, prev - 1));
      }, 1000);

      return () => clearInterval(interval);
    }
  }, [state, recordingState, status, cycleInterval]);

  // Cleanup: Cancel all timers on unmount
  useEffect(() => {
    return () => {
      timerManager.current.cancelAllTimers();
    };
  }, []);

  // Task 10: Card Interaction Functions

  /**
   * Toggle card expansion state
   * Cancels auto-collapse timer when user manually interacts
   */
  const toggleCardExpansion = (cardId: string) => {
    setCards(prevCards =>
      prevCards.map(card =>
        card.id === cardId
          ? { ...card, isExpanded: !card.isExpanded, isNew: false }
          : card
      )
    );
    // Cancel auto-collapse timer when user manually interacts
    timerManager.current.cancelTimer(cardId);
  };

  /**
   * Expand all cards
   */
  const expandAllCards = () => {
    setCards(prevCards =>
      prevCards.map(card => ({ ...card, isExpanded: true }))
    );
  };

  /**
   * Collapse all cards and cancel all timers
   */
  const collapseAllCards = () => {
    setCards(prevCards =>
      prevCards.map(card => ({ ...card, isExpanded: false }))
    );
    timerManager.current.cancelAllTimers();
  };

  /**
   * Pause auto-collapse timer when user hovers over card
   */
  const pauseAutoCollapseTimer = (cardId: string) => {
    timerManager.current.pauseTimer(cardId);
  };

  /**
   * Resume auto-collapse timer when user stops hovering
   */
  const resumeAutoCollapseTimer = (cardId: string) => {
    timerManager.current.resumeTimer(cardId);
  };

  // Helper function to format duration in seconds as "Xm Ys"
  const formatDuration = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}m ${secs}s`;
  };

  // Placeholder rendering when state is null or cycle 0
  if (!state || state.cycle_metadata.cycle_number === 0) {
    return (
      <div className="copilot-card-stack-placeholder">
        <p>Co-Pilot will start analyzing once recording begins...</p>
      </div>
    );
  }

  // Derive current cycle and stats for footer
  const currentCycle = state.cycle_metadata.cycle_number;
  const cyclesDone = currentCycle;
  const totalAudioAnalyzed = state.cycle_metadata.total_audio_seconds || 0;

  // Main render with full card stack UI
  return (
    <div className="copilot-card-stack">
      {/* Task 11: Panel Header */}
      <div className="copilot-panel-header">
        <div className="copilot-panel-title">
          <h3>Co-Pilot Agent</h3>
          <span className={`copilot-status-indicator ${runningStatus}`}>
            {runningStatus === 'recording' && <span className="pulse-dot" />}
            {runningStatus === 'complete' && <span className="checkmark">✓</span>}
          </span>
        </div>
        <div className="copilot-bulk-actions">
          <button onClick={expandAllCards}>Expand All</button>
          <span className="separator">|</span>
          <button onClick={collapseAllCards}>Collapse All</button>
        </div>
      </div>

      {/* Task 12: Card Stack Container */}
      <div className="copilot-card-area" ref={cardAreaRef}>
        {/* Task 13: Final Summary Card (rendered at top if exists) */}
        {finalSummaryCard && (
          <div className="copilot-final-summary-card" aria-label="Session Summary Card">
            <div className="copilot-card-header">
              <span className="copilot-summary-icon">📋</span>
              <span className="copilot-card-title">Session Summary</span>
            </div>
            <div className="copilot-card-body">
              {finalSummaryCard.summary && (
                <div className="summary-section">
                  <h5>Summary</h5>
                  <p>{finalSummaryCard.summary}</p>
                </div>
              )}
              {finalSummaryCard.keyTakeaways.length > 0 && (
                <div className="summary-section">
                  <h5>Key Takeaways</h5>
                  <ul>
                    {finalSummaryCard.keyTakeaways.map((item, i) => (
                      <li key={i}>{item}</li>
                    ))}
                  </ul>
                </div>
              )}
              {finalSummaryCard.actionItems.length > 0 && (
                <div className="summary-section">
                  <h5>Action Items</h5>
                  <ul>
                    {finalSummaryCard.actionItems.map((item, i) => (
                      <li key={i}>☐ {item}</li>
                    ))}
                  </ul>
                </div>
              )}
              {finalSummaryCard.decisions.length > 0 && (
                <div className="summary-section">
                  <h5>Decisions</h5>
                  <ul>
                    {finalSummaryCard.decisions.map((item, i) => (
                      <li key={i}>✓ {item}</li>
                    ))}
                  </ul>
                </div>
              )}
              {finalSummaryCard.openQuestions.length > 0 && (
                <div className="summary-section">
                  <h5>Open Questions</h5>
                  <ul>
                    {finalSummaryCard.openQuestions.map((item, i) => (
                      <li key={i}>? {item}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Task 12: Individual Cards */}
        {cards.map(card => (
          <div
            key={card.id}
            className={`copilot-card ${card.isNew ? 'entering' : ''} ${card.isExpanded ? 'expanded' : 'collapsed'}`}
            onMouseEnter={() => pauseAutoCollapseTimer(card.id)}
            onMouseLeave={() => resumeAutoCollapseTimer(card.id)}
          >
            <div
              className="copilot-card-header"
              onClick={() => toggleCardExpansion(card.id)}
              role="button"
              aria-expanded={card.isExpanded}
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  toggleCardExpansion(card.id);
                }
              }}
            >
              <span className={`copilot-card-chevron ${card.isExpanded ? 'expanded' : ''}`}>▸</span>
              <span className="copilot-card-title">{card.title}</span>
              <span className={`copilot-card-badge copilot-card-badge-${card.type}`}>
                {CARD_TYPE_LABEL[card.type]}
              </span>
            </div>
            <div className="copilot-card-body">{card.body}</div>
            <div className="copilot-card-metadata">
              Cycle {card.cycle} · {formatTimestamp(card.timestamp)}
            </div>
          </div>
        ))}
      </div>

      {/* Task 14: Sticky Footer */}
      <div className="copilot-sticky-footer">
        <div className="copilot-footer-status">
          <span className={`status-indicator status-${runningStatus}`}>
            {runningStatus === 'processing' && <span className="pulse-dot" />}
            {runningStatus === 'complete' && <span className="checkmark">✓</span>}
          </span>
          <span className="status-text">
            {runningStatus === 'recording' && `Cycle ${currentCycle} in ~${nextCycleIn}s`}
            {runningStatus === 'processing' && `Processing cycle ${currentCycle}...`}
            {runningStatus === 'complete' && `Session complete`}
          </span>
        </div>
        <div className="copilot-footer-stats">
          {cyclesDone} cycles · {formatDuration(totalAudioAnalyzed)}
        </div>
      </div>
    </div>
  );
}
