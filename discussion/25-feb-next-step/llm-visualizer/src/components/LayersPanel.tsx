import { useState, useMemo } from "react";
import type { Token } from "../lib/mockTokenizer";
import {
  getLayerOutputs,
  NUM_LAYERS,
  DIMENSION_LABELS,
} from "../lib/mockModel";

interface LayersPanelProps {
  tokens: Token[];
  selectedTokenIndex: number | null;
}

export default function LayersPanel({
  tokens,
  selectedTokenIndex,
}: LayersPanelProps) {
  const [currentLayer, setCurrentLayer] = useState(0);

  const layerOutputs = useMemo(() => {
    if (tokens.length === 0) return [];
    return getLayerOutputs(tokens);
  }, [tokens]);

  if (tokens.length === 0) {
    return (
      <div className="text-slate-500 text-sm italic">
        The forward pass will be visualized here...
      </div>
    );
  }

  const tokenIndex =
    selectedTokenIndex !== null ? selectedTokenIndex : 0;
  const token = tokens[tokenIndex];

  const currentEmbedding = layerOutputs[currentLayer]?.embeddings[tokenIndex] || [];
  const prevEmbedding =
    currentLayer > 0
      ? layerOutputs[currentLayer - 1]?.embeddings[tokenIndex] || []
      : [];

  const maxAbs = Math.max(...currentEmbedding.map(Math.abs), 0.01);

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        The embeddings flow through <strong className="text-slate-200">{NUM_LAYERS} layers</strong>,
        one after another. Each layer looks at the surrounding words and adjusts the numbers.
        Use the slider to step through layers and watch how{" "}
        <strong className="text-slate-200">"{token.text}"</strong>'s numbers change.
      </p>

      {/* Layer slider */}
      <div className="mb-4">
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs text-slate-500">
            {currentLayer === 0 ? "Raw embedding (no layers yet)" : `After Layer ${currentLayer}`}
          </span>
          <span className="text-xs text-slate-500">
            Layer {currentLayer} / {NUM_LAYERS}
          </span>
        </div>
        <input
          type="range"
          min={0}
          max={NUM_LAYERS}
          value={currentLayer}
          onChange={(e) => setCurrentLayer(Number(e.target.value))}
          className="w-full h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
        />
        {/* Layer markers */}
        <div className="flex justify-between mt-1 px-0.5">
          {Array.from({ length: NUM_LAYERS + 1 }, (_, i) => (
            <button
              key={i}
              onClick={() => setCurrentLayer(i)}
              className={`w-6 h-6 rounded text-[10px] transition-colors ${
                i === currentLayer
                  ? "bg-blue-500 text-white"
                  : "bg-slate-700 text-slate-400 hover:bg-slate-600"
              }`}
            >
              {i}
            </button>
          ))}
        </div>
      </div>

      {/* Embedding values at this layer */}
      <div className="space-y-1.5">
        {currentEmbedding.map((value, i) => {
          const width = Math.abs(value) / maxAbs;
          const isPositive = value >= 0;
          const changed = currentLayer > 0 && prevEmbedding[i] !== undefined;
          const delta = changed ? value - prevEmbedding[i] : 0;

          return (
            <div key={i} className="flex items-center gap-2">
              <span className="text-[10px] text-slate-500 w-20 text-right shrink-0">
                {DIMENSION_LABELS[i]}
              </span>

              <div className="flex-1 h-5 relative bg-slate-800 rounded overflow-hidden">
                <div className="absolute left-1/2 top-0 bottom-0 w-px bg-slate-600" />
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

              <span
                className={`text-xs font-mono w-12 text-right ${
                  isPositive ? "text-emerald-400" : "text-rose-400"
                }`}
              >
                {value > 0 ? "+" : ""}
                {value.toFixed(2)}
              </span>

              {/* Change indicator */}
              {changed && Math.abs(delta) > 0.005 && (
                <span
                  className={`text-[10px] font-mono w-14 text-right ${
                    delta > 0 ? "text-emerald-600" : "text-rose-600"
                  }`}
                >
                  {delta > 0 ? "+" : ""}
                  {delta.toFixed(2)}
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* Layer pipeline visualization */}
      <div className="mt-4 flex items-center gap-1 overflow-x-auto py-2">
        {Array.from({ length: NUM_LAYERS + 1 }, (_, i) => (
          <div key={i} className="flex items-center">
            <div
              className={`px-2 py-1 rounded text-[10px] whitespace-nowrap cursor-pointer transition-colors ${
                i === currentLayer
                  ? "bg-blue-500/30 border border-blue-400 text-blue-300"
                  : i < currentLayer
                  ? "bg-slate-600/30 border border-slate-600 text-slate-400"
                  : "bg-slate-800 border border-slate-700 text-slate-600"
              }`}
              onClick={() => setCurrentLayer(i)}
            >
              {i === 0 ? "Embed" : `Layer ${i}`}
            </div>
            {i < NUM_LAYERS && (
              <span
                className={`mx-0.5 text-xs ${
                  i < currentLayer ? "text-slate-400" : "text-slate-700"
                }`}
              >
                →
              </span>
            )}
          </div>
        ))}
      </div>

      {/* Insight based on current layer */}
      <div className="mt-3 p-3 bg-purple-500/10 border border-purple-500/20 rounded-lg text-xs text-purple-300">
        {currentLayer === 0 && (
          <>
            <strong>Layer 0 (raw embedding):</strong> This is the dumb lookup —
            same numbers every time for "{token.text}", regardless of context.
          </>
        )}
        {currentLayer === 1 && (
          <>
            <strong>Layer 1:</strong> The first layer starts looking at neighboring
            words. The numbers are beginning to shift slightly based on what words
            are nearby.
          </>
        )}
        {currentLayer > 1 && currentLayer < NUM_LAYERS && (
          <>
            <strong>Layer {currentLayer}:</strong> Each layer refines the numbers
            further. The influence of the surrounding context gets stronger. Notice
            how the values keep shifting compared to the raw embedding.
          </>
        )}
        {currentLayer === NUM_LAYERS && (
          <>
            <strong>Final layer:</strong> After passing through all {NUM_LAYERS} layers,
            the numbers now encode the full meaning of "{token.text}" <em>in this specific
            sentence</em>. These final numbers are what gets used to predict the next word.
          </>
        )}
      </div>
    </div>
  );
}
