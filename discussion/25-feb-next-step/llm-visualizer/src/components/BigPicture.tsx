import { useState } from "react";
import { tokenize } from "../lib/mockTokenizer";
import { getEmbedding, getPredictions } from "../lib/mockModel";

const EXAMPLES = [
  "What is Rust?",
  "Rust is a programming",
  "The cat sat on",
  "Hello world",
  "I love",
];

// Reusable black box component
function BlackBox({
  label,
  hint,
  color,
  onClick,
  size = "normal",
}: {
  label: string;
  hint: string;
  color: string;
  onClick: () => void;
  size?: "normal" | "small";
}) {
  const sizeClasses =
    size === "small"
      ? "w-28 h-20 rounded-xl text-xs"
      : "w-36 h-28 rounded-2xl text-sm";

  return (
    <button
      onClick={onClick}
      className={`${sizeClasses} bg-gradient-to-br ${color}
                 border-2 border-white/10 flex flex-col items-center justify-center
                 cursor-pointer hover:border-white/25 hover:scale-105
                 transition-all duration-300 shrink-0`}
    >
      <span className="text-white font-bold">{label}</span>
      <span className="text-white/40 text-[9px] mt-1">{hint}</span>
    </button>
  );
}

function Arrow() {
  return <span className="text-slate-500 text-lg shrink-0 mx-1">→</span>;
}

// Data pill (shows a piece of data flowing between boxes)
function DataPill({
  children,
  color = "slate",
}: {
  children: React.ReactNode;
  color?: string;
}) {
  const colorMap: Record<string, string> = {
    slate: "bg-slate-800 text-slate-300 border-slate-700",
    blue: "bg-blue-500/10 text-blue-300 border-blue-500/20",
    amber: "bg-amber-500/10 text-amber-300 border-amber-500/20",
    purple: "bg-purple-500/10 text-purple-300 border-purple-500/20",
    emerald: "bg-emerald-500/10 text-emerald-300 border-emerald-500/20",
  };
  return (
    <div
      className={`px-3 py-1.5 rounded-lg border text-xs font-mono shrink-0 ${colorMap[color]}`}
    >
      {children}
    </div>
  );
}

/*
  Zoom levels:
  0 — Text → [LLM] → Next word
  1 — Text → [Tokenizer] → tokens → [The Brain] → Next word
  2 — Text → tokens → [Embedding Table] → embeddings → [Layer Stack] → predictions → [Pick Winner] → Next word
  3 — Text → tokens → embeddings → [Layer with attention heads inside] → ... → predictions → Next word
*/

export default function BigPicture() {
  const [input, setInput] = useState("Rust is a programming");
  const [zoom, setZoom] = useState(0);

  const tokens = tokenize(input);
  const predictions = getPredictions(tokens);
  const topPrediction = predictions[0];

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        {zoom === 0 &&
          "This is the simplest view. Text goes in, the next word comes out. Click the black box to see what's inside."}
        {zoom === 1 &&
          "The first thing that happens: your text gets split into tokens. Then the tokens go into the brain. Click the black box to go deeper."}
        {zoom === 2 &&
          "Tokens get looked up in a table to become embeddings (lists of numbers). Then they pass through layers. Click a box to zoom in further."}
        {zoom === 3 &&
          "Inside each layer: the embedding gets split across 32 attention heads. Each head looks at which words matter to which. Then the results get glued back together."}
      </p>

      {/* Input */}
      <div className="mb-6">
        <label className="text-xs text-slate-500 block mb-1.5">
          Your text:
        </label>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          className="w-full px-4 py-3 bg-slate-800 border border-slate-600 rounded-lg
                     text-white font-mono text-lg focus:outline-none focus:border-blue-500
                     transition-colors"
          placeholder="Type something..."
        />
        <div className="flex flex-wrap gap-2 mt-2">
          {EXAMPLES.map((ex) => (
            <button
              key={ex}
              onClick={() => setInput(ex)}
              className={`text-xs px-3 py-1 rounded-full border transition-colors cursor-pointer ${
                input === ex
                  ? "bg-blue-500/20 border-blue-400 text-blue-300"
                  : "bg-slate-800 border-slate-700 text-slate-400 hover:border-slate-500"
              }`}
            >
              {ex}
            </button>
          ))}
        </div>
      </div>

      {/* ── ZOOM LEVEL 0: Text → [LLM] → Word ── */}
      {zoom === 0 && (
        <div className="flex items-center justify-center gap-3 my-8 animate-fade-in">
          <DataPill>"{input}"</DataPill>
          <Arrow />
          <BlackBox
            label="LLM"
            hint="tap to open"
            color="from-indigo-900/60 to-purple-900/60"
            onClick={() => setZoom(1)}
          />
          <Arrow />
          <DataPill color="emerald">"{topPrediction?.token}"</DataPill>
        </div>
      )}

      {/* ── ZOOM LEVEL 1: Text → [Tokenizer] → tokens → [Brain] → Word ── */}
      {zoom === 1 && (
        <div className="my-8 animate-fade-in">
          <div className="flex items-center justify-center gap-2 flex-wrap">
            <DataPill>"{input}"</DataPill>
            <Arrow />
            <BlackBox
              label="Tokenizer"
              hint="splits text"
              color="from-blue-900/60 to-cyan-900/60"
              size="small"
              onClick={() => setZoom(1)} // already open
            />
            <Arrow />

            {/* Show actual tokens */}
            <div className="flex gap-1 flex-wrap">
              {tokens.map((t, i) => (
                <span
                  key={i}
                  className="px-2 py-1 bg-blue-500/10 border border-blue-500/20
                             rounded text-xs font-mono text-blue-300"
                >
                  "{t.text}"
                  <span className="text-blue-500/40 ml-1">#{t.id}</span>
                </span>
              ))}
            </div>

            <Arrow />
            <BlackBox
              label="The Brain"
              hint="tap to open"
              color="from-purple-900/60 to-pink-900/60"
              onClick={() => setZoom(2)}
            />
            <Arrow />
            <DataPill color="emerald">"{topPrediction?.token}"</DataPill>
          </div>

          {/* Explanation */}
          <div className="mt-6 p-3 bg-blue-500/5 border border-blue-500/15 rounded-lg text-xs text-blue-300">
            The tokenizer split your text into {tokens.length} tokens. Each token got a number (ID)
            from the vocabulary — a fixed lookup table of ~32,000 entries. Now these tokens go into
            the brain. <strong>Click "The Brain"</strong> to see what happens next.
          </div>
        </div>
      )}

      {/* ── ZOOM LEVEL 2: tokens → [Embed] → embeddings → [Layer Stack] → [Pick] → Word ── */}
      {zoom === 2 && (
        <div className="my-8 animate-fade-in">
          <div className="flex items-center justify-center gap-2 flex-wrap">
            {/* Tokens coming in */}
            <div className="flex gap-1 flex-wrap">
              {tokens.slice(0, 4).map((t, i) => (
                <span
                  key={i}
                  className="px-1.5 py-0.5 bg-blue-500/10 border border-blue-500/20
                             rounded text-[10px] font-mono text-blue-300"
                >
                  #{t.id}
                </span>
              ))}
              {tokens.length > 4 && (
                <span className="text-[10px] text-slate-500">+{tokens.length - 4}</span>
              )}
            </div>

            <Arrow />
            <BlackBox
              label="Embed Table"
              hint="ID → numbers"
              color="from-amber-900/60 to-orange-900/60"
              size="small"
              onClick={() => setZoom(2)}
            />
            <Arrow />

            {/* Show embedding preview */}
            <div className="px-2 py-1 bg-amber-500/10 border border-amber-500/20 rounded text-[10px] font-mono text-amber-300 shrink-0">
              {tokens.length} × [
              {getEmbedding(tokens[0]?.id || 0)
                .slice(0, 3)
                .map((v) => v.toFixed(1))
                .join(", ")}
              , ...]
            </div>

            <Arrow />
            <BlackBox
              label="32 Layers"
              hint="tap to open"
              color="from-purple-900/60 to-violet-900/60"
              onClick={() => setZoom(3)}
            />
            <Arrow />

            {/* Prediction mini */}
            <div className="flex flex-col gap-0.5 shrink-0">
              {predictions.slice(0, 3).map((p, i) => (
                <div key={i} className="flex items-center gap-1">
                  <span
                    className={`text-[10px] font-mono ${
                      i === 0 ? "text-emerald-300" : "text-slate-500"
                    }`}
                  >
                    "{p.token}" {(p.probability * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
            </div>

            <Arrow />
            <DataPill color="emerald">"{topPrediction?.token}"</DataPill>
          </div>

          {/* Explanation */}
          <div className="mt-6 p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
            Each token ID gets looked up in the embedding table — a giant list that converts
            each number into a list of 4,096 numbers. Same ID always gives the same list (dumb lookup).
            Then these lists of numbers pass through 32 layers.{" "}
            <strong>Click "32 Layers"</strong> to see what happens inside.
          </div>
        </div>
      )}

      {/* ── ZOOM LEVEL 3: Inside a layer — attention heads ── */}
      {zoom === 3 && (
        <div className="my-6 animate-fade-in">
          <div className="text-xs text-slate-400 mb-3 text-center">
            Inside each of the 32 layers, this happens:
          </div>

          {/* The layer interior */}
          <div className="p-4 bg-purple-500/5 border border-purple-500/20 rounded-xl">
            {/* Input */}
            <div className="flex items-center justify-center gap-2 mb-4">
              <DataPill color="amber">4,096 numbers per token</DataPill>
              <Arrow />
              <span className="text-xs text-slate-400">split into 32 chunks</span>
              <Arrow />
              <span className="text-xs text-purple-300 font-mono">32 × 128</span>
            </div>

            {/* Attention heads grid */}
            <div className="flex items-center justify-center gap-2 mb-4">
              <div className="grid grid-cols-8 gap-1.5">
                {Array.from({ length: 32 }, (_, i) => (
                  <div
                    key={i}
                    className="w-9 h-9 bg-purple-500/15 border border-purple-500/25
                               rounded-lg flex items-center justify-center
                               text-[9px] text-purple-300 font-mono"
                  >
                    H{i + 1}
                  </div>
                ))}
              </div>
            </div>

            <div className="text-[10px] text-slate-500 text-center mb-4">
              Each head gets 128 numbers. It looks at all the words and decides which ones
              matter to which. Each head focuses on something different.
            </div>

            {/* Output */}
            <div className="flex items-center justify-center gap-2">
              <span className="text-xs text-purple-300 font-mono">32 × 128</span>
              <Arrow />
              <span className="text-xs text-slate-400">glue back together</span>
              <Arrow />
              <DataPill color="purple">4,096 numbers (refined)</DataPill>
            </div>
          </div>

          {/* Layer repeat */}
          <div className="mt-4 flex items-center justify-center gap-1">
            {Array.from({ length: 32 }, (_, i) => (
              <div
                key={i}
                className={`h-4 rounded-sm transition-all ${
                  i === 0
                    ? "w-4 bg-purple-500/40 border border-purple-400/50"
                    : "w-2 bg-purple-500/15 border border-purple-500/20"
                }`}
              />
            ))}
          </div>
          <div className="text-[10px] text-slate-500 text-center mt-1">
            This repeats 32 times. Layer 1 output feeds into Layer 2, and so on. Each layer refines the numbers further.
          </div>

          {/* Final explanation */}
          <div className="mt-4 p-3 bg-purple-500/5 border border-purple-500/15 rounded-lg text-xs text-purple-300">
            After all 32 layers, the numbers have been transformed from "generic word meanings"
            into "what should come next in this specific sentence." The model compares the final
            numbers against every word in the vocabulary and picks the most likely one.
          </div>
        </div>
      )}

      {/* Zoom controls */}
      <div className="flex items-center justify-center gap-3 mt-6">
        <div className="flex gap-1">
          {[0, 1, 2, 3].map((level) => (
            <button
              key={level}
              onClick={() => setZoom(level)}
              className={`w-8 h-8 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                zoom === level
                  ? "bg-slate-700 text-white border border-slate-500"
                  : "bg-slate-800 text-slate-500 border border-slate-700 hover:border-slate-600"
              }`}
            >
              {level}
            </button>
          ))}
        </div>
        <span className="text-[10px] text-slate-600">
          Zoom level — {zoom === 0 && "the big picture"}
          {zoom === 1 && "tokenization"}
          {zoom === 2 && "embeddings + layers"}
          {zoom === 3 && "inside a layer"}
        </span>
      </div>

      {/* Zoom out hint */}
      {zoom > 0 && (
        <div className="mt-2 text-center">
          <button
            onClick={() => setZoom(zoom - 1)}
            className="text-xs text-slate-500 hover:text-slate-400 cursor-pointer transition-colors"
          >
            ← zoom out
          </button>
        </div>
      )}
    </div>
  );
}
