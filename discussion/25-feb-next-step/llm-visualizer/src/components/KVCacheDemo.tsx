import { useState, useCallback, useRef } from "react";
import { tokenize } from "../lib/mockTokenizer";
import { getPredictions } from "../lib/mockModel";

interface PassInfo {
  passNumber: number;
  inputTokens: string[];
  newToken: string; // the one token we actually need to process
  cachedTokens: string[]; // tokens whose K/V we already have
  winner: string;
  cacheSize: number; // number of entries in KV cache after this pass
  withoutCacheWork: number; // tokens processed without cache
  withCacheWork: number; // tokens processed with cache (always 1 after first)
}

export default function KVCacheDemo() {
  const [startText, setStartText] = useState("Rust is a");
  const [passes, setPasses] = useState<PassInfo[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [currentPass, setCurrentPass] = useState(-1);
  const [showWithCache, setShowWithCache] = useState(true);
  const stopRef = useRef(false);

  const EXAMPLES = ["Rust is a", "What is", "Hello", "The cat", "I love"];
  const MAX_PASSES = 6;

  // Per-token KV cache size for a 7B model (in KB)
  const KV_PER_TOKEN_KB = 524;

  const runGeneration = useCallback(async () => {
    setPasses([]);
    setCurrentPass(-1);
    setIsRunning(true);
    stopRef.current = false;

    let currentInput = startText;
    const newPasses: PassInfo[] = [];

    for (let i = 0; i < MAX_PASSES; i++) {
      if (stopRef.current) break;

      const tokens = tokenize(currentInput);
      const tokenTexts = tokens.map((t) => t.text);
      const predictions = getPredictions(tokens);
      const winner = predictions[0]?.token || ".";

      const pass: PassInfo = {
        passNumber: i + 1,
        inputTokens: tokenTexts,
        newToken: i === 0 ? tokenTexts.join(" ") : tokenTexts[tokenTexts.length - 1] || "",
        cachedTokens: i === 0 ? [] : tokenTexts.slice(0, -1),
        winner,
        cacheSize: tokenTexts.length, // after this pass, all tokens are cached
        withoutCacheWork: tokenTexts.length,
        withCacheWork: i === 0 ? tokenTexts.length : 1,
      };

      newPasses.push(pass);
      setPasses([...newPasses]);
      setCurrentPass(i);

      await new Promise((r) => setTimeout(r, 1500));
      if (stopRef.current) break;

      currentInput = currentInput + " " + winner;
    }

    setIsRunning(false);
  }, [startText]);

  const stop = () => {
    stopRef.current = true;
    setIsRunning(false);
  };

  const reset = () => {
    stopRef.current = true;
    setIsRunning(false);
    setPasses([]);
    setCurrentPass(-1);
  };

  // Totals
  const totalWithout = passes.reduce((s, p) => s + p.withoutCacheWork, 0);
  const totalWith = passes.reduce((s, p) => s + p.withCacheWork, 0);
  const cacheSizeKB = passes.length > 0 ? passes[passes.length - 1].cacheSize * KV_PER_TOKEN_KB : 0;

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        Without the KV cache, the model would re-process{" "}
        <strong className="text-slate-200">every single token</strong> at every
        pass. The KV cache stores the "notes" (Keys and Values) from previous
        tokens so only the{" "}
        <strong className="text-slate-200">new token</strong> needs to be
        processed.
      </p>

      {/* Input */}
      <div className="mb-4">
        <label className="text-xs text-slate-500 block mb-1.5">
          Starting text:
        </label>
        <div className="flex gap-2">
          <input
            type="text"
            value={startText}
            onChange={(e) => {
              setStartText(e.target.value);
              reset();
            }}
            disabled={isRunning}
            className="flex-1 px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg
                       text-white font-mono focus:outline-none focus:border-blue-500
                       transition-colors disabled:opacity-50"
          />
          {!isRunning ? (
            <button
              onClick={runGeneration}
              className="px-5 py-2 bg-emerald-600 hover:bg-emerald-500 text-white
                         rounded-lg transition-colors cursor-pointer text-sm font-medium"
            >
              Generate →
            </button>
          ) : (
            <button
              onClick={stop}
              className="px-5 py-2 bg-rose-600 hover:bg-rose-500 text-white
                         rounded-lg transition-colors cursor-pointer text-sm font-medium"
            >
              Stop
            </button>
          )}
        </div>
        <div className="flex flex-wrap gap-2 mt-2">
          {EXAMPLES.map((ex) => (
            <button
              key={ex}
              onClick={() => {
                setStartText(ex);
                reset();
              }}
              disabled={isRunning}
              className={`text-xs px-3 py-1 rounded-full border transition-colors cursor-pointer
                          disabled:opacity-50 ${
                startText === ex
                  ? "bg-blue-500/20 border-blue-400 text-blue-300"
                  : "bg-slate-800 border-slate-700 text-slate-400 hover:border-slate-500"
              }`}
            >
              {ex}
            </button>
          ))}
        </div>
      </div>

      {/* Toggle: with/without cache view */}
      {passes.length > 0 && (
        <div className="flex gap-2 mb-4">
          <button
            onClick={() => setShowWithCache(false)}
            className={`px-4 py-2 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
              !showWithCache
                ? "bg-rose-500/20 border border-rose-500/40 text-rose-300"
                : "bg-slate-800 border border-slate-700 text-slate-400 hover:border-slate-600"
            }`}
          >
            Without KV Cache (wasteful)
          </button>
          <button
            onClick={() => setShowWithCache(true)}
            className={`px-4 py-2 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
              showWithCache
                ? "bg-emerald-500/20 border border-emerald-500/40 text-emerald-300"
                : "bg-slate-800 border border-slate-700 text-slate-400 hover:border-slate-600"
            }`}
          >
            With KV Cache (smart)
          </button>
        </div>
      )}

      {/* Pass-by-pass visualization */}
      {passes.length > 0 && (
        <div className="space-y-3">
          <div className="text-xs text-slate-400 font-medium">
            {showWithCache
              ? "Each pass — cached tokens are skipped (green = cached, blue = processed):"
              : "Each pass — ALL tokens re-processed every time (red = wasted work):"}
          </div>

          {passes.map((pass, i) => {
            const isActive = i === currentPass && isRunning;

            return (
              <div
                key={i}
                className={`p-3 rounded-lg border transition-all duration-500 ${
                  isActive
                    ? "bg-slate-800/80 border-emerald-500/30 scale-[1.01]"
                    : i <= currentPass
                    ? "bg-slate-800/30 border-slate-700"
                    : "bg-slate-800/10 border-slate-800 opacity-50"
                }`}
              >
                {/* Pass header */}
                <div className="flex items-center gap-3 mb-2">
                  <span
                    className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold shrink-0 ${
                      isActive
                        ? "bg-emerald-500/20 border border-emerald-500/40 text-emerald-300"
                        : "bg-slate-700 border border-slate-600 text-slate-400"
                    }`}
                  >
                    {pass.passNumber}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="text-[10px] text-slate-500 mb-0.5">
                      Pass {pass.passNumber}
                      {showWithCache
                        ? ` — process ${pass.withCacheWork} token${pass.withCacheWork > 1 ? "s" : ""}, reuse ${pass.cachedTokens.length} from cache`
                        : ` — process ALL ${pass.withoutCacheWork} tokens`}
                    </div>
                  </div>
                </div>

                {/* Token visualization */}
                <div className="ml-10 flex flex-wrap gap-1 mb-2">
                  {pass.inputTokens.map((tok, j) => {
                    const isCached =
                      showWithCache && i > 0 && j < pass.inputTokens.length - 1;
                    const isNew = !isCached;
                    const isWasted = !showWithCache && i > 0 && j < pass.inputTokens.length - 1;

                    return (
                      <span
                        key={j}
                        className={`px-2 py-1 rounded text-[11px] font-mono border transition-all ${
                          isCached
                            ? "bg-emerald-500/10 border-emerald-500/20 text-emerald-400/60"
                            : isWasted
                            ? "bg-rose-500/10 border-rose-500/20 text-rose-300"
                            : "bg-blue-500/15 border-blue-500/25 text-blue-300"
                        }`}
                      >
                        {tok}
                        {isCached && (
                          <span className="text-[8px] text-emerald-500/50 ml-1">
                            cached
                          </span>
                        )}
                        {isWasted && (
                          <span className="text-[8px] text-rose-400/50 ml-1">
                            redo
                          </span>
                        )}
                      </span>
                    );
                  })}
                </div>

                {/* Arrow to result */}
                <div className="ml-10 flex items-center gap-2">
                  <span className="text-slate-600 text-[10px]">
                    {showWithCache ? "→ 32 layers (1 token)" : `→ 32 layers (${pass.withoutCacheWork} tokens)`}
                  </span>
                  <span className="text-slate-600 text-[10px]">→</span>
                  <span className="text-[11px] font-mono font-bold text-emerald-400">
                    picks "{pass.winner}"
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Comparison stats */}
      {passes.length > 0 && !isRunning && (
        <div className="mt-4 grid grid-cols-2 gap-3">
          {/* Without cache */}
          <div className="p-3 bg-rose-500/5 border border-rose-500/15 rounded-lg">
            <div className="text-xs font-medium text-rose-300 mb-2">
              Without KV Cache
            </div>
            <div className="text-2xl font-bold text-rose-400 font-mono">
              {totalWithout}
            </div>
            <div className="text-[10px] text-rose-300/60 mt-1">
              total tokens processed
            </div>
            <div className="mt-2 text-[10px] text-slate-500">
              {passes.map((p) => p.withoutCacheWork).join(" + ")} ={" "}
              {totalWithout} token computations
            </div>
          </div>

          {/* With cache */}
          <div className="p-3 bg-emerald-500/5 border border-emerald-500/15 rounded-lg">
            <div className="text-xs font-medium text-emerald-300 mb-2">
              With KV Cache
            </div>
            <div className="text-2xl font-bold text-emerald-400 font-mono">
              {totalWith}
            </div>
            <div className="text-[10px] text-emerald-300/60 mt-1">
              total tokens processed
            </div>
            <div className="mt-2 text-[10px] text-slate-500">
              {passes.map((p) => p.withCacheWork).join(" + ")} = {totalWith}{" "}
              token computations
            </div>
          </div>
        </div>
      )}

      {/* Savings */}
      {passes.length > 0 && !isRunning && totalWithout > 0 && (
        <div className="mt-3 p-3 bg-blue-500/5 border border-blue-500/15 rounded-lg text-xs text-blue-300">
          <strong>Savings:</strong> The KV cache avoided{" "}
          <strong>{totalWithout - totalWith}</strong> redundant token
          computations — that's{" "}
          <strong>
            {Math.round(((totalWithout - totalWith) / totalWithout) * 100)}%
          </strong>{" "}
          less work!
        </div>
      )}

      {/* Cache memory cost */}
      {passes.length > 0 && !isRunning && (
        <div className="mt-3 p-3 bg-amber-500/5 border border-amber-500/15 rounded-lg text-xs text-amber-300">
          <strong>The trade-off:</strong> The KV cache now holds notes for{" "}
          <strong>{passes[passes.length - 1].cacheSize} tokens</strong>,
          using ~<strong>{(cacheSizeKB / 1024).toFixed(1)} MB</strong> of RAM
          (524 KB per token for a 7B model).
          <div className="mt-2 text-[10px] text-amber-300/60">
            Scale this up:{" "}
            <span className="text-amber-300/80">4,096 tokens → ~2 GB</span>
            {" · "}
            <span className="text-amber-300/80">32k tokens → ~16 GB</span>
            {" · "}
            <span className="text-amber-300/80">128k tokens → ~65 GB</span>
            {" — more RAM than the model itself!"}
          </div>
        </div>
      )}

      {/* Cache growth visualization */}
      {passes.length > 1 && !isRunning && (
        <div className="mt-4">
          <div className="text-xs text-slate-400 font-medium mb-2">
            KV Cache growing with each pass:
          </div>
          <div className="flex items-end gap-2 h-20">
            {passes.map((pass, i) => {
              const maxSize = passes[passes.length - 1].cacheSize;
              const heightPct = (pass.cacheSize / maxSize) * 100;
              return (
                <div key={i} className="flex flex-col items-center flex-1">
                  <div
                    className="w-full bg-gradient-to-t from-amber-500/30 to-amber-500/10 border border-amber-500/20 rounded-t transition-all"
                    style={{ height: `${heightPct}%` }}
                  />
                  <div className="text-[9px] text-slate-500 mt-1">
                    P{pass.passNumber}
                  </div>
                  <div className="text-[9px] text-amber-400/60 font-mono">
                    {pass.cacheSize}
                  </div>
                </div>
              );
            })}
          </div>
          <div className="text-[10px] text-slate-500 mt-1 text-center">
            Cache entries (tokens stored) after each pass
          </div>
        </div>
      )}

      {/* Reset */}
      {passes.length > 0 && !isRunning && (
        <div className="mt-3 text-center">
          <button
            onClick={reset}
            className="text-xs text-slate-500 hover:text-slate-400 cursor-pointer transition-colors"
          >
            Reset and try again
          </button>
        </div>
      )}
    </div>
  );
}
