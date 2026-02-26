import { useMemo } from "react";
import type { Token } from "../lib/mockTokenizer";
import { getPredictions } from "../lib/mockModel";
import type { Prediction } from "../lib/mockModel";

interface PredictionPanelProps {
  tokens: Token[];
}

export default function PredictionPanel({ tokens }: PredictionPanelProps) {
  const predictions: Prediction[] = useMemo(() => {
    if (tokens.length === 0) return [];
    return getPredictions(tokens);
  }, [tokens]);

  if (tokens.length === 0) {
    return (
      <div className="text-slate-500 text-sm italic">
        Next token predictions will appear here...
      </div>
    );
  }

  const maxProb = Math.max(...predictions.map((p) => p.probability));
  const winnerIndex = predictions.findIndex(
    (p) => p.probability === maxProb
  );

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        After all the layers, the model outputs a{" "}
        <strong className="text-slate-200">probability for every token</strong> in the
        vocabulary (all 32,000 of them). Here are the top 10 most likely next tokens.
        The model picks the highest one.
      </p>

      {/* Input context */}
      <div className="mb-4 p-2 bg-slate-800/50 rounded">
        <span className="text-[10px] text-slate-500 block mb-1">
          Given the text:
        </span>
        <span className="text-sm font-mono text-slate-300">
          "{tokens.map((t) => t.text).join(" ")}"
        </span>
        <span className="text-sm text-slate-500 ml-1">→ next word is...</span>
      </div>

      {/* Probability bars */}
      <div className="space-y-1.5">
        {predictions.map((pred, i) => {
          const width = (pred.probability / maxProb) * 100;
          const isWinner = i === winnerIndex;

          return (
            <div
              key={i}
              className={`flex items-center gap-2 p-1.5 rounded transition-all ${
                isWinner
                  ? "bg-emerald-500/10 border border-emerald-500/30"
                  : ""
              }`}
            >
              {/* Rank */}
              <span className="text-[10px] text-slate-600 w-4 text-right">
                {i + 1}
              </span>

              {/* Token name */}
              <span
                className={`font-mono text-sm w-24 shrink-0 ${
                  isWinner ? "text-emerald-300 font-bold" : "text-slate-300"
                }`}
              >
                "{pred.token}"
              </span>

              {/* Bar */}
              <div className="flex-1 h-5 bg-slate-800 rounded overflow-hidden">
                <div
                  className={`h-full rounded transition-all duration-700 ${
                    isWinner ? "bg-emerald-500/50" : "bg-blue-500/30"
                  }`}
                  style={{ width: `${width}%` }}
                />
              </div>

              {/* Percentage */}
              <span
                className={`text-xs font-mono w-14 text-right ${
                  isWinner ? "text-emerald-400 font-bold" : "text-slate-400"
                }`}
              >
                {(pred.probability * 100).toFixed(1)}%
              </span>

              {/* Winner badge */}
              {isWinner && (
                <span className="text-[10px] bg-emerald-500/30 text-emerald-300 px-1.5 py-0.5 rounded">
                  PICK
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* Explanation */}
      <div className="mt-4 p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-xs text-emerald-300">
        <strong>What happens next:</strong> The model picks "
        {predictions[winnerIndex]?.token}" (
        {((predictions[winnerIndex]?.probability || 0) * 100).toFixed(1)}%
        probability), appends it to the text, and runs the{" "}
        <em>entire process again</em> to predict the next token after that. This
        repeats until the model generates a stop token or hits the max length.
        One token at a time.
      </div>

      {/* Vocabulary note */}
      <div className="mt-2 text-[10px] text-slate-600">
        Showing top 10 of 32,000 possible tokens. The remaining 31,990 tokens
        share the leftover{" "}
        {(
          (1 - predictions.reduce((s, p) => s + p.probability, 0)) *
          100
        ).toFixed(1)}
        % probability.
      </div>
    </div>
  );
}
