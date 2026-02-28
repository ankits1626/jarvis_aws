/**
 * Utility functions for Co-Pilot Card Stack UX
 * 
 * This module provides helper functions for card ID generation, title extraction,
 * timestamp formatting, and card type mapping.
 */

import { CoPilotCardType, CoPilotState, CoPilotCard, FinalSummaryCard } from '../state/types';

/**
 * Generate a deterministic card ID
 * 
 * @param cycle - Cycle number
 * @param type - Card type
 * @param index - Index within the type array
 * @returns Card ID in format "cycle-{N}-{type}-{index}"
 */
export function generateCardId(cycle: number, type: string, index: number): string {
  return `cycle-${cycle}-${type}-${index}`;
}

/**
 * Extract a short title from card body text
 * 
 * Extracts the first clause before comma, period, or dash, and caps at 60 characters.
 * 
 * @param body - Full card body text
 * @returns Extracted title (max 60 chars with ellipsis if truncated)
 */
export function extractTitle(body: string): string {
  if (!body || body.trim().length === 0) {
    return '';
  }

  // Find first clause before comma, period, dash, or em-dash
  const match = body.match(/^([^,.\-—]+)/);
  const title = match ? match[1].trim() : body.trim();

  // Cap at 60 characters with ellipsis
  if (title.length > 60) {
    return title.substring(0, 57) + '...';
  }

  return title;
}

/**
 * Format Unix timestamp to readable time
 * 
 * @param timestamp - Unix timestamp in seconds
 * @returns Formatted time string (e.g., "2:34 PM")
 */
export function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleTimeString('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true
  });
}

/**
 * Map CoPilotState fields to card types
 */
export const CARD_TYPE_MAP: Record<string, CoPilotCardType> = {
  key_points: 'insight',
  decisions: 'decision',
  action_items: 'action_item',
  open_questions: 'question',
  running_summary: 'summary_update',
};

/**
 * Priority ordering for cards within a single cycle
 * Lower number = higher priority (appears first)
 */
export const CARD_TYPE_PRIORITY: Record<CoPilotCardType, number> = {
  decision: 1,
  action_item: 2,
  question: 3,
  insight: 4,
  summary_update: 5,
};

/**
 * Human-readable labels for card types
 */
export const CARD_TYPE_LABEL: Record<CoPilotCardType, string> = {
  insight: 'Insight',
  decision: 'Decision',
  action_item: 'Action',
  question: 'Question',
  summary_update: 'Summary',
};

/**
 * Create cards from CoPilotState diff
 * 
 * Compares new CoPilotState against previous state and existing cards to identify
 * genuinely new items. Returns an array of new cards sorted by priority.
 * 
 * @param newState - New CoPilotState from backend
 * @param oldState - Previous CoPilotState (null if first update)
 * @param existingCards - Array of existing cards
 * @returns Array of new CoPilotCard objects sorted by priority
 */
export function createCardsFromStateDiff(
  newState: CoPilotState,
  oldState: CoPilotState | null,
  existingCards: CoPilotCard[]
): CoPilotCard[] {
  const newCards: CoPilotCard[] = [];
  const cycle = newState.cycle_metadata.cycle_number;
  
  // Parse timestamp with NaN guard - fallback to current time if malformed
  const parsed = new Date(newState.cycle_metadata.last_updated_at).getTime();
  const timestamp = isNaN(parsed) ? Math.floor(Date.now() / 1000) : Math.floor(parsed / 1000);

  // Helper to check if item already exists in existing cards
  const existsInCards = (body: string): boolean => {
    return existingCards.some(card => card.body === body);
  };

  // Check key_points for new insights
  newState.key_points.forEach((point, index) => {
    if (!existsInCards(point)) {
      newCards.push({
        id: generateCardId(cycle, 'insight', index),
        type: 'insight',
        title: extractTitle(point),
        body: point,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });

  // Check decisions for new decision cards
  newState.decisions.forEach((decision, index) => {
    if (!existsInCards(decision)) {
      newCards.push({
        id: generateCardId(cycle, 'decision', index),
        type: 'decision',
        title: extractTitle(decision),
        body: decision,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });

  // Check action_items for new action cards
  newState.action_items.forEach((item, index) => {
    if (!existsInCards(item)) {
      newCards.push({
        id: generateCardId(cycle, 'action_item', index),
        type: 'action_item',
        title: extractTitle(item),
        body: item,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });

  // Check open_questions for new question cards
  newState.open_questions.forEach((question, index) => {
    if (!existsInCards(question)) {
      newCards.push({
        id: generateCardId(cycle, 'question', index),
        type: 'question',
        title: extractTitle(question),
        body: question,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });

  // Check running_summary for changes (create summary_update card if changed)
  if (!oldState || newState.running_summary !== oldState.running_summary) {
    if (newState.running_summary && newState.running_summary.trim().length > 0) {
      newCards.push({
        id: generateCardId(cycle, 'summary_update', 0),
        type: 'summary_update',
        title: extractTitle(newState.running_summary),
        body: newState.running_summary,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  }

  // Sort by priority (decision > action_item > question > insight > summary_update)
  newCards.sort((a, b) => CARD_TYPE_PRIORITY[a.type] - CARD_TYPE_PRIORITY[b.type]);

  return newCards;
}



/**
 * Timer state for auto-collapse management
 */
export interface TimerState {
  /** Timeout handle */
  timeout: ReturnType<typeof setTimeout> | null;
  
  /** Timestamp when timer started (milliseconds) */
  startedAt: number;
  
  /** Total duration in milliseconds */
  duration: number;
  
  /** Remaining time in milliseconds */
  remaining: number;
}

/**
 * Auto-collapse timer manager
 * 
 * Manages timers for auto-collapsing cards with pause/resume support.
 * Should be stored in a useRef to avoid stale closures.
 */
export class AutoCollapseTimerManager {
  private timers: Map<string, TimerState> = new Map();
  private onCollapseCallback: (cardId: string) => void;

  constructor(onCollapse: (cardId: string) => void) {
    this.onCollapseCallback = onCollapse;
  }

  /**
   * Start auto-collapse timer for a card
   * 
   * @param cardId - Card ID
   * @param duration - Duration in seconds
   */
  startTimer(cardId: string, duration: number): void {
    // Cancel existing timer if any
    this.cancelTimer(cardId);

    const durationMs = duration * 1000;
    const timeout = setTimeout(() => {
      this.onCollapseCallback(cardId);
      this.timers.delete(cardId);
    }, durationMs);

    this.timers.set(cardId, {
      timeout,
      startedAt: Date.now(),
      duration: durationMs,
      remaining: durationMs,
    });
  }

  /**
   * Pause auto-collapse timer for a card
   * 
   * @param cardId - Card ID
   */
  pauseTimer(cardId: string): void {
    const timerState = this.timers.get(cardId);
    if (timerState && timerState.timeout) {
      clearTimeout(timerState.timeout);
      const elapsed = Date.now() - timerState.startedAt;
      timerState.remaining = timerState.duration - elapsed;
      timerState.timeout = null;
    }
  }

  /**
   * Resume auto-collapse timer for a card
   * 
   * @param cardId - Card ID
   */
  resumeTimer(cardId: string): void {
    const timerState = this.timers.get(cardId);
    if (timerState && !timerState.timeout && timerState.remaining > 0) {
      const timeout = setTimeout(() => {
        this.onCollapseCallback(cardId);
        this.timers.delete(cardId);
      }, timerState.remaining);

      timerState.timeout = timeout;
      timerState.startedAt = Date.now();
      timerState.duration = timerState.remaining;
    }
  }

  /**
   * Cancel auto-collapse timer for a card
   * 
   * @param cardId - Card ID
   */
  cancelTimer(cardId: string): void {
    const timerState = this.timers.get(cardId);
    if (timerState) {
      if (timerState.timeout) {
        clearTimeout(timerState.timeout);
      }
      this.timers.delete(cardId);
    }
  }

  /**
   * Cancel all timers (cleanup on unmount)
   */
  cancelAllTimers(): void {
    this.timers.forEach(timerState => {
      if (timerState.timeout) {
        clearTimeout(timerState.timeout);
      }
    });
    this.timers.clear();
  }
}

/**
 * Create final summary card from CoPilotState
 * 
 * Extracts all accumulated data from the final state to create a comprehensive
 * summary card displayed when recording stops.
 * 
 * @param state - Final CoPilotState when recording stops
 * @returns FinalSummaryCard with all non-empty sections
 */
export function createFinalSummaryCard(state: CoPilotState): FinalSummaryCard {
  return {
    summary: state.running_summary || '',
    keyTakeaways: [...state.key_points],
    actionItems: [...state.action_items],
    decisions: [...state.decisions],
    openQuestions: [...state.open_questions],
  };
}
