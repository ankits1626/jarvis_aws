import type { Token } from "../lib/mockTokenizer";

const TOKEN_COLORS = [
  "bg-blue-500/20 border-blue-400",
  "bg-emerald-500/20 border-emerald-400",
  "bg-amber-500/20 border-amber-400",
  "bg-purple-500/20 border-purple-400",
  "bg-rose-500/20 border-rose-400",
  "bg-cyan-500/20 border-cyan-400",
  "bg-orange-500/20 border-orange-400",
  "bg-indigo-500/20 border-indigo-400",
];

interface TokenizerPanelProps {
  tokens: Token[];
  selectedTokenIndex: number | null;
  onSelectToken: (index: number) => void;
}

export default function TokenizerPanel({
  tokens,
  selectedTokenIndex,
  onSelectToken,
}: TokenizerPanelProps) {
  if (tokens.length === 0) {
    return (
      <div className="text-slate-500 text-sm italic">
        Type something above to see tokens appear here...
      </div>
    );
  }

  // Group subword tokens by their original word
  let colorIndex = 0;
  const tokenColors: string[] = [];
  let lastOriginal: string | undefined;

  tokens.forEach((token) => {
    if (token.isSubword) {
      if (token.originalWord !== lastOriginal) {
        colorIndex = (colorIndex + 1) % TOKEN_COLORS.length;
        lastOriginal = token.originalWord;
      }
    } else {
      colorIndex = (colorIndex + 1) % TOKEN_COLORS.length;
      lastOriginal = undefined;
    }
    tokenColors.push(TOKEN_COLORS[colorIndex]);
  });

  return (
    <div>
      {/* Explanation */}
      <p className="text-slate-400 text-sm mb-4">
        Your text gets split into <strong className="text-slate-200">tokens</strong> —
        word pieces the model can understand. Each token gets a number (ID) from the vocabulary lookup table.
        Common words get their own token. Rare words get{" "}
        <span className="text-amber-400">split into smaller pieces</span>.
      </p>

      {/* Token display */}
      <div className="flex flex-wrap gap-2">
        {tokens.map((token, i) => (
          <button
            key={i}
            onClick={() => onSelectToken(i)}
            className={`
              relative group border rounded-lg px-3 pt-2 pb-5
              transition-all duration-200 cursor-pointer
              ${tokenColors[i]}
              ${selectedTokenIndex === i ? "ring-2 ring-white scale-105" : "hover:scale-105"}
              ${token.isSubword ? "border-dashed" : ""}
            `}
          >
            {/* Token text */}
            <span className="text-sm font-mono text-slate-100">
              {token.text === " " ? "⎵" : `"${token.text}"`}
            </span>

            {/* Token ID */}
            <span className="absolute bottom-1 left-1/2 -translate-x-1/2 text-[10px] text-slate-400 font-mono">
              #{token.id}
            </span>

            {/* Subword indicator */}
            {token.isSubword && (
              <span className="absolute -top-2 -right-2 text-[9px] bg-amber-500/30 text-amber-300 px-1 rounded">
                piece
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Stats */}
      <div className="mt-4 flex gap-4 text-xs text-slate-500">
        <span>{tokens.length} tokens total</span>
        <span>{tokens.filter((t) => t.isSubword).length} subword pieces</span>
        <span>Vocabulary size: 32,000</span>
      </div>

      {/* Subword explanation if any exist */}
      {tokens.some((t) => t.isSubword) && (
        <div className="mt-3 p-3 bg-amber-500/10 border border-amber-500/20 rounded-lg text-xs text-amber-300">
          Dashed borders = subword pieces. The word{" "}
          <strong>
            "
            {tokens.find((t) => t.isSubword)?.originalWord}
            "
          </strong>{" "}
          wasn't in the vocabulary as a whole, so it was split into smaller
          pieces that are.
        </div>
      )}
    </div>
  );
}
