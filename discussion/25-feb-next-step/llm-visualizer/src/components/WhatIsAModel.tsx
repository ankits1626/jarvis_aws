import { useState } from "react";

/*
  Interactive walkthrough: What IS a model?

  Phases:
  0 — "A model is just a file full of numbers"
  1 — "These numbers are called weights"
  2 — "Inside the file — anatomy of the weight groups"
  3 — "Where do weights come from? Training."
  4 — "Bigger file = more numbers = smarter model"
*/

// Fake "weights" for visualization — need good spread of values
function seededRandom(seed: number): number {
  // Multiple rounds to get better distribution
  let s = seed;
  s = ((s * 16807) + 12345) % 2147483647;
  s = ((s * 48271) + 67890) % 2147483647;
  return (s / 2147483647) * 2 - 1; // Range: -1 to 1
}

// Pre-generate weight values so they're consistent
function getWeightValues(count: number): number[] {
  return Array.from({ length: count }, (_, i) => {
    const raw = seededRandom(i * 137 + 42);
    // Round to 4 decimal places like real weights
    return Math.round(raw * 10000) / 10000;
  });
}

function WeightGrid({ count, highlight }: { count: number; highlight?: boolean }) {
  const weights = getWeightValues(count);

  return (
    <div className="grid gap-1" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${highlight ? "70px" : "28px"}, 1fr))` }}>
      {weights.map((val, i) => {
        const intensity = Math.abs(val);
        const isPositive = val > 0;

        if (!highlight) {
          // Compact view — just colored squares
          return (
            <div
              key={i}
              className="h-7 rounded-sm bg-slate-800 border border-slate-700"
              style={{
                opacity: 0.3 + intensity * 0.7,
              }}
            />
          );
        }

        return (
          <div
            key={i}
            className={`h-8 rounded-sm flex items-center justify-center text-[9px] font-mono
                       transition-all duration-300 ${
              isPositive
                ? "bg-blue-500/25 text-blue-300 border border-blue-500/20"
                : "bg-rose-500/25 text-rose-300 border border-rose-500/20"
            }`}
            style={{
              opacity: 0.4 + intensity * 0.6,
            }}
          >
            {val > 0 ? "+" : ""}{val.toFixed(3)}
          </div>
        );
      })}
    </div>
  );
}

function TrainingStep({
  step,
  sentence,
  correct,
  guess,
  isCorrect,
  nudge,
}: {
  step: number;
  sentence: string;
  correct: string;
  guess: string;
  isCorrect: boolean;
  nudge: string;
}) {
  return (
    <div className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg">
      <div className="text-[10px] text-slate-500 mb-1">Training example #{step}</div>
      <div className="font-mono text-sm text-slate-300 mb-2">
        "{sentence} <span className="text-slate-500">___</span>"
      </div>
      <div className="flex items-center gap-3 text-xs">
        <span className="text-slate-400">
          Model guessed: <strong className={isCorrect ? "text-emerald-400" : "text-rose-400"}>"{guess}"</strong>
        </span>
        <span className="text-slate-600">|</span>
        <span className="text-slate-400">
          Correct answer: <strong className="text-emerald-300">"{correct}"</strong>
        </span>
      </div>
      <div className={`mt-2 text-[10px] ${isCorrect ? "text-emerald-400/60" : "text-amber-400/60"}`}>
        {nudge}
      </div>
    </div>
  );
}

function FileSection({
  color,
  label,
  description,
  detail,
  count,
}: {
  color: string;
  label: string;
  description: string;
  detail: string;
  count: string;
  heightPct: number;
}) {
  const colorMap: Record<string, { bg: string; border: string; text: string; badge: string }> = {
    amber: {
      bg: "bg-amber-500/10",
      border: "border-amber-500/25",
      text: "text-amber-300",
      badge: "bg-amber-500/20 text-amber-400",
    },
    purple: {
      bg: "bg-purple-500/10",
      border: "border-purple-500/25",
      text: "text-purple-300",
      badge: "bg-purple-500/20 text-purple-400",
    },
    emerald: {
      bg: "bg-emerald-500/10",
      border: "border-emerald-500/25",
      text: "text-emerald-300",
      badge: "bg-emerald-500/20 text-emerald-400",
    },
  };
  const c = colorMap[color] || colorMap.amber;

  return (
    <div className={`px-3 py-2.5 ${c.bg} border ${c.border} rounded-lg`}>
      <div className="flex items-center justify-between mb-1">
        <span className={`text-xs font-medium ${c.text}`}>{label}</span>
        <span className={`text-[9px] px-1.5 py-0.5 rounded ${c.badge} font-mono`}>
          {count}
        </span>
      </div>
      <div className="text-[10px] text-slate-400">{description}</div>
      <div className="text-[9px] text-slate-600 font-mono mt-0.5">{detail}</div>
    </div>
  );
}

function LayerSection({ layerNum }: { layerNum: number }) {
  return (
    <div className="px-3 py-2 bg-purple-500/8 border border-purple-500/20 rounded-lg">
      <div className="flex items-center justify-between mb-1">
        <span className="text-[11px] font-medium text-purple-300">
          Layer {layerNum}
        </span>
        <span className="text-[9px] px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-400 font-mono">
          ~39 million weights
        </span>
      </div>
      <div className="flex gap-1.5 ml-2">
        <div className="flex-1 px-2 py-1 bg-purple-500/10 border border-purple-500/15 rounded text-[9px]">
          <span className="text-purple-400">Attention weights</span>
          <span className="text-slate-600 ml-1">— decides which words matter</span>
        </div>
        <div className="flex-1 px-2 py-1 bg-blue-500/10 border border-blue-500/15 rounded text-[9px]">
          <span className="text-blue-400">Transform weights</span>
          <span className="text-slate-600 ml-1">— refines the numbers</span>
        </div>
      </div>
    </div>
  );
}

export default function WhatIsAModel() {
  const [phase, setPhase] = useState(0);
  const [trainingStep, setTrainingStep] = useState(0);
  const [weightsRevealed, setWeightsRevealed] = useState(false);

  const TRAINING_EXAMPLES = [
    {
      sentence: "The cat sat on the",
      correct: "mat",
      guess: "banana",
      isCorrect: false,
      nudge: "Way off! Nudge the weights so 'mat' scores higher next time...",
    },
    {
      sentence: "Rust is a programming",
      correct: "language",
      guess: "elephant",
      isCorrect: false,
      nudge: "Still wrong. Nudge more. The weights are slowly learning patterns...",
    },
    {
      sentence: "I love",
      correct: "you",
      guess: "pizza",
      isCorrect: false,
      nudge: "Not quite. But 'pizza' is more plausible than 'elephant' was — progress!",
    },
    {
      sentence: "Hello",
      correct: "world",
      guess: "there",
      isCorrect: false,
      nudge: "'there' is actually reasonable! The weights are getting closer...",
    },
    {
      sentence: "The sun rises in the",
      correct: "east",
      guess: "east",
      isCorrect: true,
      nudge: "Correct! After billions of examples, the weights learned this pattern.",
    },
  ];

  return (
    <div>
      {/* Phase navigation */}
      <div className="flex gap-2 mb-6 flex-wrap">
        {["The File", "Weights", "Inside the File", "Training", "Scale"].map((label, i) => (
          <button
            key={i}
            onClick={() => setPhase(i)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
              phase === i
                ? "bg-amber-500/20 border border-amber-500/30 text-amber-200"
                : "bg-slate-800 border border-slate-700 text-slate-400 hover:border-slate-600"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* ── Phase 0: A model is just a file ── */}
      {phase === 0 && (
        <div className="space-y-4 animate-fade-in">
          <p className="text-slate-400 text-sm">
            When someone says "I downloaded Llama 3", what did they actually download?
          </p>

          <div className="flex items-center justify-center gap-4 my-8">
            {/* File icon */}
            <div className="relative">
              <div className="w-32 h-40 bg-slate-800 border-2 border-slate-600 rounded-lg flex flex-col items-center justify-center">
                <div className="text-3xl mb-2">📄</div>
                <div className="text-xs font-mono text-slate-300 font-bold">
                  llama-3-8b.gguf
                </div>
                <div className="text-[10px] text-slate-500 mt-1">4.3 GB</div>
              </div>
            </div>

            <div className="text-slate-500 text-2xl">=</div>

            {/* What's inside */}
            <div className="w-48 h-40 bg-slate-800 border-2 border-amber-500/30 rounded-lg p-3 flex flex-col items-center justify-center">
              <div className="text-xs text-amber-400 font-medium mb-2">
                Just numbers
              </div>
              <div className="font-mono text-[10px] text-slate-400 text-center leading-relaxed">
                0.0023, -0.0891,<br />
                0.1547, -0.0234,<br />
                0.0891, -0.1234,<br />
                0.0456, 0.0789,<br />
                <span className="text-slate-600">...8 billion more</span>
              </div>
            </div>
          </div>

          <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
            <strong>That's it.</strong> A model file is just a giant list of numbers.
            Nothing else. No code, no rules, no database of facts.
            Just billions of numbers called <strong>weights</strong>.
          </div>

          <div className="text-center mt-4">
            <button
              onClick={() => setPhase(1)}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white
                         rounded-lg transition-colors cursor-pointer text-sm"
            >
              What are weights? →
            </button>
          </div>
        </div>
      )}

      {/* ── Phase 1: Weights ── */}
      {phase === 1 && (
        <div className="space-y-4 animate-fade-in">
          <p className="text-slate-400 text-sm">
            Think of weights as <strong className="text-slate-200">knobs on a mixing board</strong>.
            A music mixing board has maybe 100 knobs. An LLM has <strong className="text-slate-200">billions</strong> of them.
          </p>

          <p className="text-slate-400 text-sm">
            Each knob is set to a specific number. The combination of ALL these numbers
            determines what the model "knows." Click the button to see the raw weights:
          </p>

          <div className="text-center my-4">
            <button
              onClick={() => setWeightsRevealed(!weightsRevealed)}
              className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white
                         rounded-lg transition-colors cursor-pointer text-sm"
            >
              {weightsRevealed ? "Hide weights" : "Reveal weights"}
            </button>
          </div>

          <div className="p-4 bg-slate-800/50 border border-slate-700 rounded-lg">
            <div className="text-[10px] text-slate-500 mb-2">
              {weightsRevealed
                ? "Showing 80 out of 8,000,000,000 weights (one ten-millionth):"
                : "8 billion knobs, each set to a tiny number:"}
            </div>
            <WeightGrid count={80} highlight={weightsRevealed} />
            {weightsRevealed && (
              <div className="mt-3 text-[10px] text-slate-500">
                Blue = positive, Red = negative. Brighter = bigger number.
                Every single one of these matters — change one and the model's output changes slightly.
              </div>
            )}
          </div>

          {/* Where weights live */}
          <div className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
            <strong className="text-slate-300">Where these weights are used:</strong>
            <div className="mt-2 space-y-1">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-amber-400 shrink-0" />
                <span>The <strong className="text-amber-300">embedding table</strong> — weights that convert token IDs to number lists</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-purple-400 shrink-0" />
                <span>The <strong className="text-purple-300">attention heads</strong> — weights that decide which words matter to which</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-blue-400 shrink-0" />
                <span>The <strong className="text-blue-300">layer transforms</strong> — weights that refine the numbers at each layer</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-emerald-400 shrink-0" />
                <span>The <strong className="text-emerald-300">prediction head</strong> — weights that convert final numbers to word probabilities</span>
              </div>
            </div>
          </div>

          <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
            <strong>Key insight:</strong> The embedding table, the attention heads, the layers —
            they're ALL just weights. Different groups of numbers from the same giant file.
            The model file contains ALL of them packed together.
          </div>

          <div className="text-center mt-4">
            <button
              onClick={() => setPhase(2)}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white
                         rounded-lg transition-colors cursor-pointer text-sm"
            >
              See what's inside the file →
            </button>
          </div>
        </div>
      )}

      {/* ── Phase 2: Inside the File ── */}
      {phase === 2 && (
        <div className="space-y-4 animate-fade-in">
          <p className="text-slate-400 text-sm">
            The model file is these groups of weights <strong className="text-slate-200">packed one after another</strong>.
            When you load a model, the software reads the file and puts each group where it belongs.
          </p>

          {/* Anatomy diagram */}
          <div className="p-4 bg-slate-800/30 border border-slate-700 rounded-xl">
            <div className="text-[10px] text-slate-500 mb-3 font-mono text-center">
              llama-3-8b.gguf — what's inside:
            </div>

            <div className="space-y-1.5 max-w-lg mx-auto">
              {/* Embedding table */}
              <FileSection
                color="amber"
                label="Embedding Table"
                description="One row per word in vocabulary. Token ID → look up row → get 4,096 numbers."
                detail="32,000 rows × 4,096 columns"
                count="~131 million weights"
                heightPct={8}
              />

              {/* Layers 1-32 */}
              <div className="relative">
                <div className="space-y-0.5">
                  {/* Show first 2 layers in detail */}
                  <LayerSection layerNum={1} />
                  <LayerSection layerNum={2} />

                  {/* Collapsed middle layers */}
                  <div className="flex items-center gap-2 px-3 py-2 bg-slate-800/60 border border-slate-700/50 rounded-lg">
                    <div className="flex gap-0.5">
                      {Array.from({ length: 8 }, (_, i) => (
                        <div key={i} className="w-1 h-4 bg-slate-600 rounded-full" />
                      ))}
                    </div>
                    <span className="text-[10px] text-slate-500 italic">
                      ...layers 3 through 31 (same structure, different weight values)...
                    </span>
                  </div>

                  <LayerSection layerNum={32} />
                </div>

                {/* Brace on the right */}
                <div className="absolute -right-16 top-0 bottom-0 flex items-center">
                  <div className="text-[10px] text-slate-500 writing-mode-vertical flex flex-col items-center">
                    <span className="bg-slate-950 px-1 text-purple-400/60 whitespace-nowrap">
                      32 layers × ~39M = ~1.25B
                    </span>
                  </div>
                </div>
              </div>

              {/* Prediction head */}
              <FileSection
                color="emerald"
                label="Prediction Head"
                description="Converts the final 4,096 numbers into a probability for every word."
                detail="4,096 × 32,000"
                count="~131 million weights"
                heightPct={8}
              />
            </div>

            {/* Total */}
            <div className="mt-4 text-center">
              <div className="inline-block px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-lg">
                <span className="text-xs text-amber-300 font-medium">
                  Total: ~8 billion weights
                </span>
                <span className="text-[10px] text-amber-300/50 ml-2">
                  × 2 bytes each = ~16 GB file
                </span>
              </div>
            </div>
          </div>

          {/* How multiplication works */}
          <div className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
            <strong className="text-slate-300">How weights are used — it's just multiplication:</strong>
            <div className="mt-3 flex items-center justify-center gap-2 flex-wrap">
              <span className="px-2 py-1 bg-blue-500/10 border border-blue-500/20 rounded text-blue-300 font-mono text-[10px]">
                input numbers
              </span>
              <span className="text-slate-500">×</span>
              <span className="px-2 py-1 bg-amber-500/10 border border-amber-500/20 rounded text-amber-300 font-mono text-[10px]">
                weights
              </span>
              <span className="text-slate-500">=</span>
              <span className="px-2 py-1 bg-emerald-500/10 border border-emerald-500/20 rounded text-emerald-300 font-mono text-[10px]">
                output numbers
              </span>
            </div>
            <div className="mt-2 text-[10px] text-slate-500 text-center">
              At every step in the pipeline, "processing" means: multiply input by weights, get output.
              That's literally all a neural network does. Multiply, add, repeat.
            </div>
          </div>

          {/* Key insight */}
          <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
            <strong>To generate a single token</strong>, the model must read through ALL of these groups —
            the embedding table, all 32 layers, and the prediction head. That's why the entire
            file must fit in RAM. If any part is missing, the model can't work.
          </div>

          <div className="text-center mt-4">
            <button
              onClick={() => setPhase(3)}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white
                         rounded-lg transition-colors cursor-pointer text-sm"
            >
              Where do these numbers come from? →
            </button>
          </div>
        </div>
      )}

      {/* ── Phase 3: Training ── */}
      {phase === 3 && (
        <div className="space-y-4 animate-fade-in">
          <p className="text-slate-400 text-sm">
            Training is how these billions of numbers get their values.
            It works like flash cards — <strong className="text-slate-200">show the model a sentence
            with the last word hidden, and ask it to guess</strong>.
          </p>

          <p className="text-slate-400 text-sm">
            At first, all 8 billion weights are <strong className="text-slate-200">random</strong>,
            so the model guesses randomly. But after each wrong guess, the weights get
            nudged slightly to be less wrong next time.
          </p>

          {/* Training simulation */}
          <div className="space-y-2">
            {TRAINING_EXAMPLES.slice(0, trainingStep + 1).map((ex, i) => (
              <TrainingStep key={i} step={i + 1} {...ex} />
            ))}
          </div>

          {trainingStep < TRAINING_EXAMPLES.length - 1 ? (
            <div className="text-center">
              <button
                onClick={() => setTrainingStep(trainingStep + 1)}
                className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white
                           rounded-lg transition-colors cursor-pointer text-sm"
              >
                Next training example →
              </button>
              <div className="text-[10px] text-slate-600 mt-1">
                Example {trainingStep + 1} of {TRAINING_EXAMPLES.length}
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div className="p-3 bg-emerald-500/5 border border-emerald-500/15 rounded-lg text-xs text-emerald-300">
                <strong>That's training in a nutshell.</strong> Repeat this process
                trillions of times with text from the entire internet. After weeks
                of computation on thousands of GPUs, those 8 billion random numbers
                have been nudged into 8 billion very specific numbers that happen to
                be great at predicting the next word.
              </div>

              <div className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
                <strong className="text-slate-300">The training cost:</strong>
                <div className="mt-2 grid grid-cols-3 gap-2 text-center">
                  <div>
                    <div className="text-lg font-bold text-amber-400">15T</div>
                    <div className="text-[10px] text-slate-500">tokens of text<br />(trillions)</div>
                  </div>
                  <div>
                    <div className="text-lg font-bold text-purple-400">16K</div>
                    <div className="text-[10px] text-slate-500">GPUs running<br />for weeks</div>
                  </div>
                  <div>
                    <div className="text-lg font-bold text-emerald-400">$M+</div>
                    <div className="text-[10px] text-slate-500">millions of<br />dollars</div>
                  </div>
                </div>
              </div>

              <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
                <strong>When you download a model file</strong>, you're downloading the
                result of all that training — the final, tuned weights. You get the
                "answer key" without paying for the training. That's why open-source
                models are such a big deal.
              </div>
            </div>
          )}
        </div>
      )}

      {/* ── Phase 4: Scale ── */}
      {phase === 4 && (
        <div className="space-y-4 animate-fade-in">
          <p className="text-slate-400 text-sm">
            More weights = more "knobs" = more patterns the model can learn.
            That's why bigger models are smarter — but they also need more memory.
          </p>

          {/* Model comparison */}
          <div className="space-y-2">
            {[
              { name: "Llama 3 8B", params: "8B", weights: 8, size: 16, bar: 20, quality: "Good for simple tasks" },
              { name: "Llama 3 70B", params: "70B", weights: 70, size: 140, bar: 60, quality: "Very capable" },
              { name: "Llama 3 405B", params: "405B", weights: 405, size: 810, bar: 100, quality: "Near frontier" },
            ].map((m) => (
              <div
                key={m.name}
                className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg"
              >
                <div className="flex items-center justify-between mb-2">
                  <div>
                    <span className="text-sm font-medium text-white">{m.name}</span>
                    <span className="text-xs text-slate-500 ml-2">
                      {m.params} parameters
                    </span>
                  </div>
                  <span className="text-xs text-slate-400">{m.size} GB (FP16)</span>
                </div>
                {/* Size bar */}
                <div className="h-3 bg-slate-700 rounded-full overflow-hidden mb-1.5">
                  <div
                    className="h-full bg-gradient-to-r from-amber-500/60 to-amber-500/30 rounded-full transition-all"
                    style={{ width: `${m.bar}%` }}
                  />
                </div>
                <div className="flex justify-between text-[10px]">
                  <span className="text-slate-500">
                    {m.weights} billion weights × 2 bytes each
                  </span>
                  <span className="text-amber-400/60">{m.quality}</span>
                </div>
              </div>
            ))}
          </div>

          <div className="p-3 bg-slate-800/50 border border-slate-700 rounded-lg text-xs text-slate-400">
            <strong className="text-slate-300">Why "B" means billions of weights:</strong>
            <div className="mt-2">
              "8B" = 8 billion weights. Each weight is typically stored as a 16-bit number (2 bytes).
              So 8B × 2 bytes = 16 GB. That's the minimum RAM you need to load the entire model.
            </div>
          </div>

          <div className="p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
            <strong>This is why you hear about RAM so much.</strong> To generate
            even a single token, the model must read through ALL its weights.
            If 16 GB of weights don't fit in your RAM, they spill to disk,
            and everything becomes 100x slower. This is the core problem that
            quantization (Part 3) solves — shrinking these numbers to fit.
          </div>
        </div>
      )}
    </div>
  );
}
