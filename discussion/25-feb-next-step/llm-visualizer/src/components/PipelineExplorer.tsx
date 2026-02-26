import { useState, useMemo } from "react";
import { tokenize } from "../lib/mockTokenizer";
import {
  getEmbedding,
  getPredictions,
  getLayerOutputs,
  EMBEDDING_DIM,
} from "../lib/mockModel";

/*
  Pipeline Explorer — shows the model as a black box at each step.

  3 levels:
    Closed  → step card: input → [black box] → output
    Open    → black box expands to show which model components are used
    Detail  → clicking a component shows actual data visualization
*/

const EXAMPLES = [
  "Rust is a programming",
  "What is Rust?",
  "The cat sat on",
  "Hello world",
  "I love",
];

// ─── Shared small components ───

function DataPill({
  children,
  color = "slate",
  small = false,
}: {
  children: React.ReactNode;
  color?: string;
  small?: boolean;
}) {
  const colorMap: Record<string, string> = {
    slate: "bg-slate-800 text-slate-300 border-slate-700",
    blue: "bg-blue-500/10 text-blue-300 border-blue-500/20",
    amber: "bg-amber-500/10 text-amber-300 border-amber-500/20",
    purple: "bg-purple-500/10 text-purple-300 border-purple-500/20",
    emerald: "bg-emerald-500/10 text-emerald-300 border-emerald-500/20",
    rose: "bg-rose-500/10 text-rose-300 border-rose-500/20",
  };
  return (
    <div
      className={`${small ? "px-2 py-1 text-[10px]" : "px-3 py-1.5 text-xs"} rounded-lg border font-mono shrink-0 ${colorMap[color]}`}
    >
      {children}
    </div>
  );
}

function Arrow() {
  return <span className="text-slate-500 text-lg shrink-0 mx-1">→</span>;
}

// ─── Component button inside an opened black box ───

function ComponentButton({
  label,
  sublabel,
  color,
  isWeight,
  onClick,
  isActive,
}: {
  label: string;
  sublabel: string;
  color: string;
  isWeight: boolean;
  onClick: () => void;
  isActive: boolean;
}) {
  const colorMap: Record<string, string> = {
    cyan: "from-cyan-900/60 to-blue-900/60 border-cyan-500/30",
    amber: "from-amber-900/60 to-orange-900/60 border-amber-500/30",
    purple: "from-purple-900/60 to-violet-900/60 border-purple-500/30",
    emerald: "from-emerald-900/60 to-teal-900/60 border-emerald-500/30",
  };
  return (
    <button
      onClick={onClick}
      className={`px-3 py-2 rounded-xl bg-gradient-to-br ${colorMap[color]}
                 border-2 cursor-pointer hover:scale-[1.03] transition-all
                 flex flex-col items-center text-center ${isActive ? "ring-2 ring-white/30 scale-[1.03]" : ""}`}
    >
      <span className="text-xs font-bold text-white">{label}</span>
      <span className="text-[9px] text-white/40 mt-0.5">{sublabel}</span>
      {isWeight ? (
        <span className="text-[8px] mt-1 px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-400">
          weights
        </span>
      ) : (
        <span className="text-[8px] mt-1 px-1.5 py-0.5 rounded bg-slate-500/20 text-slate-400">
          rules (not weights)
        </span>
      )}
    </button>
  );
}

// ─── Main component ───

export default function PipelineExplorer() {
  const [input, setInput] = useState("Rust is a programming");
  const [openStep, setOpenStep] = useState<number | null>(null);
  const [detailView, setDetailView] = useState<string | null>(null);

  const tokens = useMemo(() => tokenize(input), [input]);
  const predictions = useMemo(() => getPredictions(tokens), [tokens]);
  const layerOutputs = useMemo(() => getLayerOutputs(tokens), [tokens]);
  const topPrediction = predictions[0];

  const toggleStep = (step: number) => {
    if (openStep === step) {
      setOpenStep(null);
      setDetailView(null);
    } else {
      setOpenStep(step);
      setDetailView(null);
    }
  };

  const toggleDetail = (view: string) => {
    setDetailView(detailView === view ? null : view);
  };

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        Follow your text through <strong className="text-slate-200">every step of the pipeline</strong>.
        Each step uses part of the model. Click the black boxes to see what's inside.
        Click a component to see the actual data.
      </p>

      {/* Input */}
      <div className="mb-6">
        <label className="text-xs text-slate-500 block mb-1.5">Your text:</label>
        <input
          type="text"
          value={input}
          onChange={(e) => {
            setInput(e.target.value);
            setOpenStep(null);
            setDetailView(null);
          }}
          className="w-full px-4 py-3 bg-slate-800 border border-slate-600 rounded-lg
                     text-white font-mono text-lg focus:outline-none focus:border-blue-500
                     transition-colors"
          placeholder="Type something..."
        />
        <div className="flex flex-wrap gap-2 mt-2">
          {EXAMPLES.map((ex) => (
            <button
              key={ex}
              onClick={() => {
                setInput(ex);
                setOpenStep(null);
                setDetailView(null);
              }}
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

      {/* Pipeline steps */}
      <div className="space-y-3">
        {/* ── STEP 1: Tokenize ── */}
        <StepCard
          stepNum={1}
          title="Tokenize"
          subtitle="Split text into pieces"
          isOpen={openStep === 1}
          onToggle={() => toggleStep(1)}
          input={<DataPill>"{input}"</DataPill>}
          output={
            <DataPill color="blue">
              [{tokens.slice(0, 3).map((t) => `"${t.text}"`).join(", ")}
              {tokens.length > 3 ? ", ..." : ""}]
            </DataPill>
          }
        >
          {/* Opened: show tokenizer component */}
          <div className="space-y-3">
            <div className="flex justify-center">
              <ComponentButton
                label="Tokenizer"
                sublabel="vocabulary lookup"
                color="cyan"
                isWeight={false}
                onClick={() => toggleDetail("tokenizer")}
                isActive={detailView === "tokenizer"}
              />
            </div>
            <div className="text-[10px] text-slate-500 text-center">
              The tokenizer is a separate small file (~1 MB). It's a dictionary, not weights.
              It was built once, not learned during training.
            </div>

            {/* Detail: tokenizer internals */}
            {detailView === "tokenizer" && (
              <div className="p-3 bg-cyan-500/5 border border-cyan-500/15 rounded-lg animate-fade-in">
                <div className="text-xs text-cyan-300 font-medium mb-2">
                  Vocabulary lookup for "{input}":
                </div>
                <div className="space-y-1">
                  {tokens.map((t, i) => (
                    <div key={i} className="flex items-center gap-2 text-[11px] font-mono">
                      <span className="text-slate-400">"{t.text}"</span>
                      <span className="text-slate-600">→</span>
                      <span className="text-cyan-400">#{t.id}</span>
                      {t.isSubword && (
                        <span className="text-[9px] px-1.5 py-0.5 rounded bg-cyan-500/10 text-cyan-400/60">
                          subword piece of "{t.originalWord}"
                        </span>
                      )}
                    </div>
                  ))}
                </div>
                <div className="mt-2 text-[10px] text-slate-500">
                  Each word gets looked up in a ~32,000 word dictionary. Same word always gets same ID.
                </div>
              </div>
            )}
          </div>
        </StepCard>

        {/* ── STEP 2: Embed ── */}
        <StepCard
          stepNum={2}
          title="Embed"
          subtitle="Convert IDs to numbers"
          isOpen={openStep === 2}
          onToggle={() => toggleStep(2)}
          input={
            <DataPill color="blue">
              [{tokens.slice(0, 3).map((t) => `#${t.id}`).join(", ")}
              {tokens.length > 3 ? ", ..." : ""}]
            </DataPill>
          }
          output={
            <DataPill color="amber">
              {tokens.length} × [4,096 numbers]
            </DataPill>
          }
        >
          <div className="space-y-3">
            <div className="flex justify-center">
              <ComponentButton
                label="Embedding Table"
                sublabel="32,000 rows × 4,096 cols"
                color="amber"
                isWeight={true}
                onClick={() => toggleDetail("embedding")}
                isActive={detailView === "embedding"}
              />
            </div>
            <div className="text-[10px] text-slate-500 text-center">
              ~131 million weights. Each token ID maps to a row of 4,096 numbers.
              Same ID always gives the exact same numbers (dumb lookup).
            </div>

            {/* Detail: embedding values */}
            {detailView === "embedding" && (
              <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg animate-fade-in">
                <div className="text-xs text-amber-300 font-medium mb-2">
                  Embedding lookup for each token:
                </div>
                <div className="space-y-2">
                  {tokens.slice(0, 4).map((t, i) => {
                    const emb = getEmbedding(t.id);
                    return (
                      <div key={i}>
                        <div className="text-[10px] text-slate-400 mb-1">
                          "{t.text}" (#{t.id}) → row {t.id} in table:
                        </div>
                        <div className="flex gap-1">
                          {emb.map((v, j) => (
                            <div
                              key={j}
                              className={`flex-1 h-8 rounded flex items-center justify-center text-[9px] font-mono ${
                                v >= 0
                                  ? "bg-blue-500/20 text-blue-300"
                                  : "bg-rose-500/20 text-rose-300"
                              }`}
                            >
                              {v.toFixed(2)}
                            </div>
                          ))}
                          <div className="flex items-center text-[9px] text-slate-600 px-1">
                            ...×{4096 - EMBEDDING_DIM} more
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
                <div className="mt-2 text-[10px] text-amber-300/60">
                  We show {EMBEDDING_DIM} dimensions. A real model uses 4,096 per token.
                  Same token ID always produces the same numbers — no context yet.
                </div>
              </div>
            )}
          </div>
        </StepCard>

        {/* ── STEP 3: Process (32 Layers) ── */}
        <StepCard
          stepNum={3}
          title="Process"
          subtitle="32 layers refine the numbers"
          isOpen={openStep === 3}
          onToggle={() => toggleStep(3)}
          input={
            <DataPill color="amber">
              {tokens.length} × [4,096 raw numbers]
            </DataPill>
          }
          output={
            <DataPill color="purple">
              {tokens.length} × [4,096 refined numbers]
            </DataPill>
          }
        >
          <div className="space-y-3">
            <div className="text-[10px] text-slate-500 text-center mb-2">
              Each of the 32 layers uses two groups of weights:
            </div>
            <div className="flex justify-center gap-3">
              <ComponentButton
                label="Attention Weights"
                sublabel="which words matter?"
                color="purple"
                isWeight={true}
                onClick={() => toggleDetail("attention")}
                isActive={detailView === "attention"}
              />
              <ComponentButton
                label="Transform Weights"
                sublabel="refine the numbers"
                color="purple"
                isWeight={true}
                onClick={() => toggleDetail("transform")}
                isActive={detailView === "transform"}
              />
            </div>
            <div className="text-[10px] text-slate-500 text-center">
              32 layers × (~6M attention + ~33M transform) = ~1.25 billion weights.
              This is where "Rust" starts to mean different things in different sentences.
            </div>

            {/* Detail: attention */}
            {detailView === "attention" && (
              <div className="p-3 bg-purple-500/5 border border-purple-500/15 rounded-lg animate-fade-in">
                <div className="text-xs text-purple-300 font-medium mb-2">
                  Attention — how much each word looks at every other word:
                </div>
                {/* Mini attention grid */}
                <div className="overflow-x-auto">
                  <div className="inline-block">
                    <div className="flex">
                      <div className="w-16 shrink-0" />
                      {tokens.slice(0, 5).map((t, j) => (
                        <div key={j} className="w-12 text-center text-[9px] text-slate-400 font-mono shrink-0">
                          {t.text.length > 5 ? t.text.slice(0, 4) + "…" : t.text}
                        </div>
                      ))}
                    </div>
                    {tokens.slice(0, 5).map((fromT, i) => (
                      <div key={i} className="flex items-center">
                        <div className="w-16 text-right pr-2 text-[9px] text-slate-400 font-mono shrink-0">
                          {fromT.text.length > 6 ? fromT.text.slice(0, 5) + "…" : fromT.text}
                        </div>
                        {tokens.slice(0, 5).map((toT, j) => {
                          // Simple attention simulation
                          const dist = Math.abs(i - j);
                          const val = dist === 0 ? 0.15 : dist === 1 ? 0.5 : 0.1;
                          const total = tokens.slice(0, 5).reduce((s, _, k) => {
                            const d = Math.abs(i - k);
                            return s + (d === 0 ? 0.15 : d === 1 ? 0.5 : 0.1);
                          }, 0);
                          const norm = val / total;
                          return (
                            <div
                              key={j}
                              className="w-12 h-8 m-0.5 rounded flex items-center justify-center text-[9px] font-mono shrink-0"
                              style={{ backgroundColor: `rgba(147, 51, 234, ${norm * 1.5})` }}
                            >
                              <span className={norm > 0.25 ? "text-white" : "text-slate-500"}>
                                {(norm * 100).toFixed(0)}%
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    ))}
                  </div>
                </div>
                <div className="mt-2 text-[10px] text-purple-300/60">
                  Each row adds up to 100%. Bright = strong attention.
                  The attention weights (learned during training) determine these patterns.
                  Input numbers × attention weights = attention scores.
                </div>
              </div>
            )}

            {/* Detail: transform */}
            {detailView === "transform" && (
              <div className="p-3 bg-purple-500/5 border border-purple-500/15 rounded-lg animate-fade-in">
                <div className="text-xs text-purple-300 font-medium mb-2">
                  Transform — how numbers change through layers:
                </div>
                {tokens.length > 0 && (
                  <div className="space-y-2">
                    <div className="text-[10px] text-slate-400 mb-1">
                      "{tokens[0].text}" embedding through layers:
                    </div>
                    {layerOutputs.filter((_, i) => i === 0 || i === layerOutputs.length - 1).map((lo) => (
                      <div key={lo.layer}>
                        <div className="text-[9px] text-slate-500 mb-1">
                          {lo.layer === 0 ? "Before layers (raw embedding):" : `After all layers (refined):`}
                        </div>
                        <div className="flex gap-1">
                          {lo.embeddings[0]?.slice(0, EMBEDDING_DIM).map((v, d) => (
                            <div
                              key={d}
                              className={`flex-1 h-7 rounded flex items-center justify-center text-[8px] font-mono ${
                                v >= 0
                                  ? "bg-blue-500/20 text-blue-300"
                                  : "bg-rose-500/20 text-rose-300"
                              }`}
                            >
                              {v.toFixed(2)}
                            </div>
                          ))}
                        </div>
                      </div>
                    ))}
                    <div className="mt-1 text-[10px] text-purple-300/60">
                      Notice how the numbers changed? The layers used the transform weights
                      to mix in context from surrounding words. Multiply, add, repeat — 32 times.
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </StepCard>

        {/* ── STEP 4: Predict ── */}
        <StepCard
          stepNum={4}
          title="Predict"
          subtitle="Score every word in vocabulary"
          isOpen={openStep === 4}
          onToggle={() => toggleStep(4)}
          input={
            <DataPill color="purple">
              [4,096 refined numbers]
            </DataPill>
          }
          output={
            <DataPill color="emerald">
              "{topPrediction?.token}" ({((topPrediction?.probability || 0) * 100).toFixed(0)}%)
            </DataPill>
          }
        >
          <div className="space-y-3">
            <div className="flex justify-center">
              <ComponentButton
                label="Prediction Head"
                sublabel="4,096 × 32,000 matrix"
                color="emerald"
                isWeight={true}
                onClick={() => toggleDetail("prediction")}
                isActive={detailView === "prediction"}
              />
            </div>
            <div className="text-[10px] text-slate-500 text-center">
              ~131 million weights. Multiplies the final 4,096 numbers against every word
              in the vocabulary to get a score for each one.
            </div>

            {/* Detail: prediction probabilities */}
            {detailView === "prediction" && (
              <div className="p-3 bg-emerald-500/5 border border-emerald-500/15 rounded-lg animate-fade-in">
                <div className="text-xs text-emerald-300 font-medium mb-2">
                  Top predictions for "{input}":
                </div>
                <div className="space-y-1.5">
                  {predictions.slice(0, 8).map((p, i) => {
                    const barWidth = (p.probability / (predictions[0]?.probability || 1)) * 100;
                    return (
                      <div key={i} className="flex items-center gap-2">
                        <span className="w-16 text-right text-[10px] font-mono text-slate-400 shrink-0">
                          "{p.token}"
                        </span>
                        <div className="flex-1 h-5 bg-slate-800 rounded overflow-hidden">
                          <div
                            className={`h-full rounded transition-all ${
                              i === 0
                                ? "bg-emerald-500/40"
                                : "bg-slate-700"
                            }`}
                            style={{ width: `${barWidth}%` }}
                          />
                        </div>
                        <span className={`text-[10px] font-mono w-10 shrink-0 ${
                          i === 0 ? "text-emerald-300" : "text-slate-500"
                        }`}>
                          {(p.probability * 100).toFixed(1)}%
                        </span>
                      </div>
                    );
                  })}
                </div>
                <div className="mt-2 text-[10px] text-emerald-300/60">
                  The prediction head weights convert 4,096 numbers into 32,000 scores (one per word).
                  These scores become probabilities via softmax.
                </div>
              </div>
            )}
          </div>
        </StepCard>

        {/* ── STEP 5: Pick & Repeat ── */}
        <StepCard
          stepNum={5}
          title="Pick & Repeat"
          subtitle="Choose winner, loop back"
          isOpen={openStep === 5}
          onToggle={() => toggleStep(5)}
          input={
            <DataPill color="emerald">
              probabilities for 32,000 words
            </DataPill>
          }
          output={
            <DataPill color="rose">
              "{topPrediction?.token}" → append → run again
            </DataPill>
          }
        >
          <div className="space-y-3">
            <div className="flex justify-center">
              <div className="px-4 py-3 rounded-xl bg-slate-800 border-2 border-slate-600 text-center">
                <span className="text-xs font-bold text-white">argmax / sample</span>
                <div className="text-[9px] text-white/40 mt-0.5">just pick the highest</div>
                <span className="text-[8px] mt-1 inline-block px-1.5 py-0.5 rounded bg-slate-500/20 text-slate-400">
                  no weights needed
                </span>
              </div>
            </div>
            <div className="text-[10px] text-slate-500 text-center">
              No model weights used here — just pick the word with the highest probability.
              Then append it to the input and run the entire pipeline again for the next word.
            </div>

            {/* Loop visualization */}
            <div className="p-3 bg-rose-500/5 border border-rose-500/15 rounded-lg">
              <div className="text-xs text-rose-300 font-medium mb-2">The autoregressive loop:</div>
              <div className="space-y-1 font-mono text-[10px]">
                <div className="text-slate-400">
                  Pass 1: "<span className="text-slate-300">{input}</span>"
                  <span className="text-slate-600"> →</span>
                  <span className="text-emerald-400"> "{topPrediction?.token}"</span>
                </div>
                <div className="text-slate-400">
                  Pass 2: "<span className="text-slate-300">{input} {topPrediction?.token}</span>"
                  <span className="text-slate-600"> →</span>
                  <span className="text-emerald-400"> "..."</span>
                </div>
                <div className="text-slate-400">
                  Pass 3: "<span className="text-slate-300">{input} {topPrediction?.token} ...</span>"
                  <span className="text-slate-600"> →</span>
                  <span className="text-emerald-400"> "..."</span>
                </div>
              </div>
              <div className="mt-2 text-[10px] text-rose-300/60">
                Each pass reads ALL the weights from memory. For a 7B model, that's ~4 GB per token.
                This is why the KV cache matters — it avoids re-processing old tokens.
              </div>
            </div>
          </div>
        </StepCard>
      </div>

      {/* Summary: what parts of the model were used */}
      <div className="mt-6 p-4 bg-slate-800/30 border border-slate-700 rounded-xl">
        <div className="text-xs text-slate-300 font-medium mb-3">
          Total weights used for one token:
        </div>
        <div className="flex gap-2 flex-wrap">
          {[
            { label: "Tokenizer", size: "~1 MB", isWeight: false, color: "text-cyan-400" },
            { label: "Embedding Table", size: "~131M weights", isWeight: true, color: "text-amber-400" },
            { label: "32 Layers", size: "~7.5B weights", isWeight: true, color: "text-purple-400" },
            { label: "Prediction Head", size: "~131M weights", isWeight: true, color: "text-emerald-400" },
          ].map((item) => (
            <div
              key={item.label}
              className="px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-center flex-1 min-w-[120px]"
            >
              <div className={`text-[10px] font-medium ${item.color}`}>
                {item.label}
              </div>
              <div className="text-[9px] text-slate-500 mt-0.5">{item.size}</div>
              {!item.isWeight && (
                <div className="text-[8px] text-slate-600 mt-0.5">not weights</div>
              )}
            </div>
          ))}
        </div>
        <div className="mt-3 text-[10px] text-slate-500 text-center">
          Every single token generation reads through all ~8 billion weights.
          That's the entire model file, every time.
        </div>
      </div>
    </div>
  );
}

// ─── StepCard: a pipeline step with collapsible black box ───

function StepCard({
  stepNum,
  title,
  subtitle,
  isOpen,
  onToggle,
  input,
  output,
  children,
}: {
  stepNum: number;
  title: string;
  subtitle: string;
  isOpen: boolean;
  onToggle: () => void;
  input: React.ReactNode;
  output: React.ReactNode;
  children: React.ReactNode;
}) {
  const stepColors = [
    "", // unused index 0
    "border-cyan-500/20",
    "border-amber-500/20",
    "border-purple-500/20",
    "border-emerald-500/20",
    "border-rose-500/20",
  ];

  return (
    <div
      className={`p-4 rounded-xl border transition-all ${
        isOpen
          ? `bg-slate-800/40 ${stepColors[stepNum]}`
          : "bg-slate-800/20 border-slate-800 hover:border-slate-700"
      }`}
    >
      {/* Header row: step number + title */}
      <div className="flex items-center gap-3 mb-3">
        <span className="w-7 h-7 rounded-full bg-slate-700 border border-slate-600 flex items-center justify-center text-xs font-bold text-slate-300 shrink-0">
          {stepNum}
        </span>
        <div>
          <div className="text-sm font-medium text-white">{title}</div>
          <div className="text-[10px] text-slate-500">{subtitle}</div>
        </div>
      </div>

      {/* Closed view: input → [black box] → output */}
      <div className="flex items-center gap-2 flex-wrap justify-center">
        {input}
        <Arrow />
        <button
          onClick={onToggle}
          className={`px-4 py-2.5 rounded-xl bg-gradient-to-br from-indigo-900/60 to-purple-900/60
                     border-2 cursor-pointer hover:scale-105 transition-all ${
            isOpen
              ? "border-white/20 shadow-lg shadow-purple-500/10"
              : "border-white/10 hover:border-white/20"
          }`}
        >
          <span className="text-xs font-bold text-white">
            {isOpen ? "▼ Model" : "▶ Model"}
          </span>
          <div className="text-[8px] text-white/40 mt-0.5">
            {isOpen ? "tap to close" : "tap to open"}
          </div>
        </button>
        <Arrow />
        {output}
      </div>

      {/* Opened: show model components */}
      {isOpen && (
        <div className="mt-4 pt-4 border-t border-slate-700/50 animate-fade-in">
          {children}
        </div>
      )}
    </div>
  );
}
