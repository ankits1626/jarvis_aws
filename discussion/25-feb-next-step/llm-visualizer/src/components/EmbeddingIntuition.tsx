import { useState } from "react";

// Words the user can "rate" on simple human-understandable scales
const WORDS_TO_RATE = ["dog", "cat", "run", "happy", "Rust", "Python", "sad", "jump"];

// Human-readable scales (like a dating profile for words)
const SCALES = [
  { name: "Is it a thing?", left: "No (action/feeling)", right: "Yes (object/noun)" },
  { name: "Is it alive?", left: "No", right: "Yes" },
  { name: "Is it positive?", left: "Negative", right: "Positive" },
  { name: "Is it tech-related?", left: "Not at all", right: "Very much" },
];

// Pre-filled "correct" ratings to show after the user tries
const SUGGESTED_RATINGS: Record<string, number[]> = {
  dog:    [0.9,  0.95, 0.7,  0.0],
  cat:    [0.9,  0.95, 0.6,  0.0],
  run:    [0.1,  0.0,  0.5,  0.0],
  happy:  [0.0,  0.0,  0.9,  0.0],
  Rust:   [0.8,  0.0,  0.5,  0.95],
  Python: [0.8,  0.0,  0.5,  0.95],
  sad:    [0.0,  0.0,  0.1,  0.0],
  jump:   [0.1,  0.0,  0.6,  0.0],
};

type Phase = "intro" | "rating" | "compare" | "reveal";

export default function EmbeddingIntuition() {
  const [phase, setPhase] = useState<Phase>("intro");
  const [currentWordIndex, setCurrentWordIndex] = useState(0);
  const [userRatings, setUserRatings] = useState<Record<string, number[]>>({});
  const [showSuggested, setShowSuggested] = useState(false);

  const currentWord = WORDS_TO_RATE[currentWordIndex];
  const currentRating = userRatings[currentWord] || SCALES.map(() => 0.5);

  const updateRating = (scaleIndex: number, value: number) => {
    const newRating = [...currentRating];
    newRating[scaleIndex] = value;
    setUserRatings({ ...userRatings, [currentWord]: newRating });
  };

  const nextWord = () => {
    if (currentWordIndex < WORDS_TO_RATE.length - 1) {
      setCurrentWordIndex(currentWordIndex + 1);
    } else {
      setPhase("compare");
    }
  };

  const prevWord = () => {
    if (currentWordIndex > 0) {
      setCurrentWordIndex(currentWordIndex - 1);
    }
  };

  // Calculate simple "distance" between two words based on ratings
  const getDistance = (word1: string, word2: string): number => {
    const r1 = showSuggested ? SUGGESTED_RATINGS[word1] : (userRatings[word1] || SCALES.map(() => 0.5));
    const r2 = showSuggested ? SUGGESTED_RATINGS[word2] : (userRatings[word2] || SCALES.map(() => 0.5));
    if (!r1 || !r2) return 1;
    const sumSquares = r1.reduce((sum, v, i) => sum + (v - r2[i]) ** 2, 0);
    return Math.sqrt(sumSquares / SCALES.length);
  };

  // ── INTRO PHASE ──
  if (phase === "intro") {
    return (
      <div>
        <h3 className="text-lg font-semibold text-white mb-3">
          Let's build the intuition first
        </h3>

        <div className="space-y-4 text-sm text-slate-300">
          <p>
            Imagine you're building a dating app — but for <strong className="text-white">words</strong>.
          </p>
          <p>
            Each word needs a profile. And you want similar words to have similar profiles,
            so you can match them.
          </p>
          <p>
            How would you describe a word? You could rate it on a few scales:
          </p>

          <div className="bg-slate-800 rounded-lg p-4 space-y-2 font-mono text-xs">
            <div>"dog" → Is it a thing? <span className="text-emerald-400">YES</span> · Is it alive? <span className="text-emerald-400">YES</span> · Positive? <span className="text-emerald-400">Somewhat</span> · Tech? <span className="text-rose-400">NO</span></div>
            <div>"run" → Is it a thing? <span className="text-rose-400">NO (it's an action)</span> · Is it alive? <span className="text-rose-400">NO</span> · Positive? <span className="text-slate-400">Neutral</span> · Tech? <span className="text-rose-400">NO</span></div>
          </div>

          <p>
            Those 4 ratings ARE the embedding. That's literally all it is — <strong className="text-white">a list of
            numbers that describes a word</strong>.
          </p>
          <p>
            The only difference with a real model: instead of 4 human-readable scales, it uses
            4,096 scales that it invented during training. No human labels — just numbers that
            turned out to be useful for predicting the next word.
          </p>
          <p>
            Let's try it. You'll rate some words, then we'll see which ones end up "close" to each other.
          </p>
        </div>

        <button
          onClick={() => setPhase("rating")}
          className="mt-6 px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors cursor-pointer"
        >
          Let me try rating words →
        </button>
      </div>
    );
  }

  // ── RATING PHASE ──
  if (phase === "rating") {
    return (
      <div>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">
            Rate: "<span className="text-blue-400">{currentWord}</span>"
          </h3>
          <span className="text-xs text-slate-500">
            Word {currentWordIndex + 1} of {WORDS_TO_RATE.length}
          </span>
        </div>

        <p className="text-sm text-slate-400 mb-6">
          Drag each slider to describe this word. There's no wrong answer — go with your gut.
        </p>

        <div className="space-y-5">
          {SCALES.map((scale, i) => (
            <div key={i}>
              <div className="flex justify-between mb-1">
                <span className="text-sm text-slate-300">{scale.name}</span>
                <span className="text-xs font-mono text-slate-500">
                  {currentRating[i].toFixed(2)}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-[10px] text-rose-400 w-24 text-right shrink-0">
                  {scale.left}
                </span>
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.05}
                  value={currentRating[i]}
                  onChange={(e) => updateRating(i, parseFloat(e.target.value))}
                  className="flex-1 h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
                />
                <span className="text-[10px] text-emerald-400 w-24 shrink-0">
                  {scale.right}
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Current embedding preview */}
        <div className="mt-5 p-3 bg-slate-800/50 rounded-lg">
          <span className="text-[10px] text-slate-500">
            "{currentWord}" embedding (your ratings):
          </span>
          <div className="font-mono text-sm text-slate-300 mt-1">
            [{currentRating.map((v) => v.toFixed(2)).join(", ")}]
          </div>
        </div>

        {/* Navigation */}
        <div className="flex gap-3 mt-6">
          {currentWordIndex > 0 && (
            <button
              onClick={prevWord}
              className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-slate-300 rounded-lg transition-colors cursor-pointer"
            >
              ← Previous
            </button>
          )}
          <button
            onClick={nextWord}
            className="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors cursor-pointer"
          >
            {currentWordIndex < WORDS_TO_RATE.length - 1
              ? "Next word →"
              : "See results →"}
          </button>
        </div>

        {/* Progress dots */}
        <div className="flex gap-1.5 mt-4 justify-center">
          {WORDS_TO_RATE.map((w, i) => (
            <button
              key={w}
              onClick={() => setCurrentWordIndex(i)}
              className={`w-2 h-2 rounded-full transition-colors cursor-pointer ${
                i === currentWordIndex
                  ? "bg-blue-500"
                  : userRatings[w]
                  ? "bg-slate-500"
                  : "bg-slate-700"
              }`}
            />
          ))}
        </div>
      </div>
    );
  }

  // ── COMPARE PHASE ──
  if (phase === "compare") {
    // Find closest pairs
    const pairs: { w1: string; w2: string; dist: number }[] = [];
    for (let i = 0; i < WORDS_TO_RATE.length; i++) {
      for (let j = i + 1; j < WORDS_TO_RATE.length; j++) {
        pairs.push({
          w1: WORDS_TO_RATE[i],
          w2: WORDS_TO_RATE[j],
          dist: getDistance(WORDS_TO_RATE[i], WORDS_TO_RATE[j]),
        });
      }
    }
    pairs.sort((a, b) => a.dist - b.dist);

    const ratings = showSuggested ? SUGGESTED_RATINGS : userRatings;

    return (
      <div>
        <h3 className="text-lg font-semibold text-white mb-3">
          Your word embeddings
        </h3>

        <p className="text-sm text-slate-400 mb-4">
          Here are the "embeddings" you created. Words with similar ratings are close together —
          just like in a real model.
        </p>

        {/* Toggle between user and suggested */}
        <div className="flex gap-2 mb-4">
          <button
            onClick={() => setShowSuggested(false)}
            className={`text-xs px-3 py-1.5 rounded-full border transition-colors cursor-pointer ${
              !showSuggested
                ? "bg-blue-500/20 border-blue-400 text-blue-300"
                : "bg-slate-800 border-slate-700 text-slate-400"
            }`}
          >
            Your ratings
          </button>
          <button
            onClick={() => setShowSuggested(true)}
            className={`text-xs px-3 py-1.5 rounded-full border transition-colors cursor-pointer ${
              showSuggested
                ? "bg-blue-500/20 border-blue-400 text-blue-300"
                : "bg-slate-800 border-slate-700 text-slate-400"
            }`}
          >
            Suggested ratings
          </button>
        </div>

        {/* All embeddings */}
        <div className="space-y-2 mb-6">
          {WORDS_TO_RATE.map((word) => {
            const r = ratings[word] || SCALES.map(() => 0.5);
            return (
              <div
                key={word}
                className="flex items-center gap-3 p-2 bg-slate-800/50 rounded"
              >
                <span className="font-mono text-sm text-blue-300 w-16 shrink-0">
                  {word}
                </span>
                <div className="flex gap-1 flex-1">
                  {r.map((v, i) => (
                    <div
                      key={i}
                      className="flex-1 h-6 bg-slate-900 rounded overflow-hidden relative"
                      title={`${SCALES[i].name}: ${v.toFixed(2)}`}
                    >
                      <div
                        className={`absolute left-0 top-0 bottom-0 rounded transition-all duration-500 ${
                          v > 0.5
                            ? "bg-emerald-500/40"
                            : "bg-rose-500/40"
                        }`}
                        style={{ width: `${v * 100}%` }}
                      />
                      <span className="absolute inset-0 flex items-center justify-center text-[9px] text-slate-400">
                        {v.toFixed(1)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>

        {/* Scale legend */}
        <div className="flex gap-2 mb-4 text-[9px] text-slate-600">
          {SCALES.map((s, i) => (
            <span key={i} className="flex-1 text-center">{s.name}</span>
          ))}
        </div>

        {/* Closest pairs */}
        <div className="mb-6">
          <h4 className="text-sm font-medium text-slate-300 mb-2">
            Most similar pairs (closest embeddings):
          </h4>
          <div className="space-y-1">
            {pairs.slice(0, 5).map((p, i) => (
              <div
                key={i}
                className="flex items-center gap-2 text-sm"
              >
                <span className="font-mono text-blue-300">{p.w1}</span>
                <span className="text-slate-600">↔</span>
                <span className="font-mono text-blue-300">{p.w2}</span>
                <div className="flex-1 h-1.5 bg-slate-800 rounded overflow-hidden">
                  <div
                    className="h-full bg-emerald-500/50 rounded transition-all duration-500"
                    style={{ width: `${Math.max(5, (1 - p.dist) * 100)}%` }}
                  />
                </div>
                <span className="text-[10px] text-slate-500 font-mono w-12 text-right">
                  {(1 - p.dist).toFixed(2)} sim
                </span>
              </div>
            ))}
          </div>
        </div>

        <div className="p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg text-sm text-blue-300 mb-4">
          <strong>This is exactly what the model does.</strong> You rated 8 words on 4 scales.
          A real model rates 32,000+ words on 4,096 scales. The scales aren't chosen by
          humans — the model discovers whatever scales help it predict the next word best
          during training. But the core idea is identical: <strong>each word becomes a list
          of numbers, and similar words get similar numbers.</strong>
        </div>

        <div className="flex gap-3">
          <button
            onClick={() => {
              setPhase("rating");
              setCurrentWordIndex(0);
            }}
            className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-slate-300 rounded-lg transition-colors cursor-pointer"
          >
            ← Re-rate words
          </button>
          <button
            onClick={() => setPhase("reveal")}
            className="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors cursor-pointer"
          >
            Now show me attention heads →
          </button>
        </div>
      </div>
    );
  }

  // ── REVEAL / ATTENTION HEADS INTRO ──
  return (
    <div>
      <h3 className="text-lg font-semibold text-white mb-3">
        Why attention heads?
      </h3>

      <div className="space-y-4 text-sm text-slate-300">
        <p>
          You just rated words on 4 scales. But you were only one person with one perspective.
        </p>

        <p>
          What if we had <strong className="text-white">32 different people</strong> rate the same words,
          and each person focused on something different?
        </p>

        <div className="bg-slate-800 rounded-lg p-4 space-y-2 text-xs">
          <div className="text-slate-400">For the sentence: "The cat sat on the mat"</div>
          <div className="mt-2">
            <span className="text-purple-400">Person 1</span> focuses on: who does the action?
            <span className="text-slate-500"> → "cat" connects strongly to "sat"</span>
          </div>
          <div>
            <span className="text-amber-400">Person 2</span> focuses on: where is it?
            <span className="text-slate-500"> → "sat" connects to "on" and "mat"</span>
          </div>
          <div>
            <span className="text-emerald-400">Person 3</span> focuses on: what kind of thing?
            <span className="text-slate-500"> → "the" connects to "cat" (the cat, not a cat)</span>
          </div>
          <div>
            <span className="text-rose-400">Person 4</span> focuses on: is this a question?
            <span className="text-slate-500"> → looks for "?" or question words</span>
          </div>
          <div className="text-slate-600 mt-1">... and 28 more people, each with their own focus</div>
        </div>

        <p>
          Each person writes a <strong className="text-white">128-number summary</strong> of what they noticed.
          Combine all 32 summaries: 32 × 128 = <strong className="text-white">4,096 numbers</strong>.
        </p>

        <p>
          That's where the 4,096 comes from. It's not one big mystery list — it's{" "}
          <strong className="text-white">32 smaller lists glued together</strong>, each from a different
          "perspective" on the sentence.
        </p>

        <div className="p-4 bg-purple-500/10 border border-purple-500/20 rounded-lg text-purple-300">
          <strong>Each "person" is an attention head.</strong> They all read the same sentence,
          but each one decides independently which words are important to which other words.
          No human told them what to focus on — they figured out useful patterns during training.
        </div>

        <p className="text-slate-400">
          Switch to the <strong className="text-slate-200">Attention</strong> tab above to see this in action —
          an interactive map of which words are paying attention to which.
        </p>
      </div>
    </div>
  );
}
