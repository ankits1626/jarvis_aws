import { useState, useMemo } from "react";
import type { Token } from "../lib/mockTokenizer";

interface AttentionPanelProps {
  tokens: Token[];
}

// Simulated attention heads — each head has a "focus description" and
// generates attention weights between token pairs
const HEAD_DESCRIPTIONS = [
  { name: "Head 1", focus: "Who does the action?", color: "text-purple-400" },
  { name: "Head 2", focus: "Where / position", color: "text-amber-400" },
  { name: "Head 3", focus: "What kind of thing?", color: "text-emerald-400" },
  { name: "Head 4", focus: "Previous word", color: "text-rose-400" },
];

// Deterministic fake attention weights based on token properties
function seededRandom(seed: number): number {
  const s = ((seed * 16807 + 0) % 2147483647);
  return (s / 2147483647);
}

function generateAttentionWeights(
  tokens: Token[],
  headIndex: number
): number[][] {
  const n = tokens.length;
  const weights: number[][] = [];

  for (let i = 0; i < n; i++) {
    const row: number[] = [];
    for (let j = 0; j < n; j++) {
      let w = 0;

      if (headIndex === 0) {
        // Head 1: verbs attend to nouns (who does the action?)
        // Simulate: content words attend strongly to other content words
        if (i !== j && tokens[i].text.length > 2 && tokens[j].text.length > 2) {
          w = 0.3 + seededRandom(tokens[i].id * 31 + tokens[j].id * 7 + 1) * 0.5;
        } else if (i === j) {
          w = 0.15;
        } else {
          w = 0.05 + seededRandom(tokens[i].id + tokens[j].id) * 0.1;
        }
      } else if (headIndex === 1) {
        // Head 2: position-based — nearby words attend to each other
        const dist = Math.abs(i - j);
        if (dist === 0) w = 0.1;
        else if (dist === 1) w = 0.6;
        else if (dist === 2) w = 0.3;
        else w = 0.05;
      } else if (headIndex === 2) {
        // Head 3: "the/a" attends to the next noun, adjectives to nouns
        const small = ["the", "a", "an", "The", "A", "An", "this", "that"];
        if (small.includes(tokens[i].text) && j === i + 1) {
          w = 0.8;
        } else if (i === j) {
          w = 0.15;
        } else {
          w = 0.05 + seededRandom(tokens[i].id * 3 + tokens[j].id * 11 + 99) * 0.15;
        }
      } else {
        // Head 4: each token attends mostly to the previous token
        if (j === i - 1) w = 0.7;
        else if (i === j) w = 0.15;
        else w = 0.05 + seededRandom(tokens[i].id + tokens[j].id + 42) * 0.1;
      }

      row.push(w);
    }

    // Normalize row to sum to 1
    const sum = row.reduce((a, b) => a + b, 0);
    weights.push(row.map((v) => v / (sum || 1)));
  }

  return weights;
}

export default function AttentionPanel({ tokens }: AttentionPanelProps) {
  const [selectedHead, setSelectedHead] = useState(0);
  const [hoveredCell, setHoveredCell] = useState<{
    from: number;
    to: number;
  } | null>(null);

  const attentionWeights = useMemo(() => {
    if (tokens.length === 0) return [];
    return HEAD_DESCRIPTIONS.map((_, i) =>
      generateAttentionWeights(tokens, i)
    );
  }, [tokens]);

  if (tokens.length === 0) {
    return (
      <div className="text-slate-500 text-sm italic">
        Type a sentence to see how attention works...
      </div>
    );
  }

  const weights = attentionWeights[selectedHead] || [];
  const head = HEAD_DESCRIPTIONS[selectedHead];

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        Each <strong className="text-slate-200">attention head</strong> decides how much each
        word should pay attention to every other word. Below is a grid — each row is a word asking
        "how much should I care about each other word?" Brighter = more attention.
      </p>

      {/* Head selector */}
      <div className="flex gap-2 mb-4">
        {HEAD_DESCRIPTIONS.map((h, i) => (
          <button
            key={i}
            onClick={() => setSelectedHead(i)}
            className={`flex-1 px-3 py-2 rounded-lg border text-xs transition-colors cursor-pointer ${
              selectedHead === i
                ? "bg-slate-700 border-slate-500 text-white"
                : "bg-slate-800/50 border-slate-700 text-slate-400 hover:border-slate-600"
            }`}
          >
            <div className={`font-medium ${h.color}`}>{h.name}</div>
            <div className="text-slate-500 mt-0.5">{h.focus}</div>
          </button>
        ))}
      </div>

      <p className="text-xs text-slate-500 mb-3">
        <span className={head.color}>{head.name}</span> focuses on: <strong>{head.focus}</strong>.
        A real model has 32 heads — we show 4 to keep it simple. Each head discovers its own pattern during training.
      </p>

      {/* Attention grid */}
      <div className="overflow-x-auto">
        <div className="inline-block">
          {/* Column headers (keys — words being looked at) */}
          <div className="flex">
            <div className="w-20 shrink-0" /> {/* spacer for row labels */}
            {tokens.map((t, j) => (
              <div
                key={j}
                className="w-14 text-center text-[10px] text-slate-400 font-mono pb-1 shrink-0"
              >
                {t.text.length > 6 ? t.text.slice(0, 5) + "…" : t.text}
              </div>
            ))}
            <div className="w-4 shrink-0" />
          </div>

          {/* Rows (queries — words doing the looking) */}
          {tokens.map((fromToken, i) => (
            <div key={i} className="flex items-center">
              {/* Row label */}
              <div className="w-20 text-right pr-2 text-[10px] text-slate-400 font-mono shrink-0">
                {fromToken.text.length > 8
                  ? fromToken.text.slice(0, 7) + "…"
                  : fromToken.text}
              </div>

              {/* Attention cells */}
              {tokens.map((_, j) => {
                const value = weights[i]?.[j] || 0;
                const isHovered =
                  hoveredCell?.from === i && hoveredCell?.to === j;

                return (
                  <div
                    key={j}
                    className={`w-14 h-10 m-0.5 rounded flex items-center justify-center
                      text-[10px] font-mono cursor-default transition-all shrink-0 ${
                      isHovered ? "ring-1 ring-white" : ""
                    }`}
                    style={{
                      backgroundColor: `rgba(99, 102, 241, ${value * 0.9})`,
                    }}
                    onMouseEnter={() => setHoveredCell({ from: i, to: j })}
                    onMouseLeave={() => setHoveredCell(null)}
                  >
                    <span
                      className={
                        value > 0.3 ? "text-white" : "text-slate-500"
                      }
                    >
                      {(value * 100).toFixed(0)}%
                    </span>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>

      {/* Hover info */}
      <div className="mt-3 h-8 text-xs text-slate-400">
        {hoveredCell && tokens[hoveredCell.from] && tokens[hoveredCell.to] && (
          <span>
            "<strong className="text-slate-200">{tokens[hoveredCell.from].text}</strong>" pays{" "}
            <strong className="text-indigo-400">
              {((weights[hoveredCell.from]?.[hoveredCell.to] || 0) * 100).toFixed(1)}%
            </strong>{" "}
            attention to "
            <strong className="text-slate-200">{tokens[hoveredCell.to].text}</strong>"
          </span>
        )}
        {!hoveredCell && <span>Hover over a cell to see the attention detail</span>}
      </div>

      {/* Reading guide */}
      <div className="mt-2 p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
        <strong className="text-slate-300">How to read this grid:</strong> Each row is one word.
        The numbers in that row show how much that word "pays attention" to each other word.
        Each row adds up to 100%. Bright cells = strong connection. Try switching heads
        to see how each one focuses on different relationships.
      </div>

      {/* Key insight */}
      <div className="mt-3 p-3 bg-purple-500/10 border border-purple-500/20 rounded-lg text-xs text-purple-300">
        <strong>Remember:</strong> In a real model, there are 32 heads, each producing 128 numbers.
        32 × 128 = 4,096 — that's the embedding size we talked about earlier.
        The attention step is what mixes context in. This is where "Rust" in a programming sentence
        starts to differ from "Rust" in a corrosion sentence.
      </div>
    </div>
  );
}
