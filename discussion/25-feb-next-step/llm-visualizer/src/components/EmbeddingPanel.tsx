import type { Token } from "../lib/mockTokenizer";
import { getEmbedding, EMBEDDING_DIM, DIMENSION_LABELS } from "../lib/mockModel";

interface EmbeddingPanelProps {
  tokens: Token[];
  selectedTokenIndex: number | null;
}

export default function EmbeddingPanel({
  tokens,
  selectedTokenIndex,
}: EmbeddingPanelProps) {
  if (tokens.length === 0) {
    return (
      <div className="text-slate-500 text-sm italic">
        Tokens will be converted to embeddings here...
      </div>
    );
  }

  if (selectedTokenIndex === null) {
    return (
      <div>
        <p className="text-slate-400 text-sm mb-4">
          Each token ID gets converted into a <strong className="text-slate-200">list of numbers</strong> called
          an embedding. This list captures the meaning of the token. Click a token above to see its embedding.
        </p>
        <div className="text-slate-500 text-sm italic flex items-center gap-2">
          <span className="text-2xl">👆</span>
          Click any token in Step 1 to see its embedding
        </div>
      </div>
    );
  }

  const token = tokens[selectedTokenIndex];
  const embedding = getEmbedding(token.id);
  const maxAbs = Math.max(...embedding.map(Math.abs));

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        The token <strong className="text-slate-200">"{token.text}"</strong> (ID #{token.id}) gets looked up
        in the embedding table. It always returns the <strong className="text-slate-200">same list of numbers</strong>,
        no matter what sentence it appears in. This is just a dumb lookup — no thinking yet.
      </p>

      {/* Why 8 dimensions */}
      <div className="mb-4 p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
        <strong className="text-slate-300">Why only {EMBEDDING_DIM} numbers?</strong> Real models use <strong className="text-slate-300">4,096</strong> numbers
        per token (some use even more). We show {EMBEDDING_DIM} here because 4,096 bars wouldn't fit on screen.
        The idea is the same — each number is just called "dim 1", "dim 2", etc.
        No one knows what each number individually means. Together, the pattern captures the word's meaning.
      </div>

      {/* Visual embedding */}
      <div className="space-y-1.5">
        {embedding.map((value, i) => {
          const width = Math.abs(value) / (maxAbs || 1);
          const isPositive = value >= 0;
          return (
            <div key={i} className="flex items-center gap-2">
              {/* Dimension label */}
              <span className="text-[10px] text-slate-500 w-20 text-right shrink-0">
                {DIMENSION_LABELS[i]}
              </span>

              {/* Bar */}
              <div className="flex-1 h-5 relative bg-slate-800 rounded overflow-hidden">
                {/* Center line */}
                <div className="absolute left-1/2 top-0 bottom-0 w-px bg-slate-600" />

                {/* Value bar */}
                <div
                  className={`absolute top-0.5 bottom-0.5 rounded-sm transition-all duration-500 ${
                    isPositive ? "bg-emerald-500/60" : "bg-rose-500/60"
                  }`}
                  style={{
                    left: isPositive ? "50%" : `${50 - width * 50}%`,
                    width: `${width * 50}%`,
                  }}
                />
              </div>

              {/* Value */}
              <span
                className={`text-xs font-mono w-12 text-right ${
                  isPositive ? "text-emerald-400" : "text-rose-400"
                }`}
              >
                {value > 0 ? "+" : ""}
                {value.toFixed(2)}
              </span>
            </div>
          );
        })}
      </div>

      {/* Key insight */}
      <div className="mt-4 p-3 bg-blue-500/10 border border-blue-500/20 rounded-lg text-xs text-blue-300">
        <strong>Key insight:</strong> This embedding is always the same for token "
        {token.text}" (#{token.id}). Try the word "Rust" in "Rust is a programming language"
        and "Rust on the iron door" — the embedding is identical. The layers (Step 3) are
        what make it different based on context.
      </div>

      {/* Raw numbers */}
      <div className="mt-3 p-2 bg-slate-800/50 rounded font-mono text-[10px] text-slate-500">
        [{embedding.map((v) => v.toFixed(2)).join(", ")}]
      </div>
    </div>
  );
}
