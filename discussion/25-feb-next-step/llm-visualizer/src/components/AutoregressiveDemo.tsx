import { useState, useCallback, useRef } from "react";
import { tokenize } from "../lib/mockTokenizer";
import { getPredictions } from "../lib/mockModel";

interface Step {
  input: string;
  tokens: { text: string; id: number }[];
  predictions: { token: string; probability: number }[];
  winner: string;
}

export default function AutoregressiveDemo() {
  const [startText, setStartText] = useState("Rust is a");
  const [steps, setSteps] = useState<Step[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [currentStep, setCurrentStep] = useState(-1);
  const stopRef = useRef(false);

  const EXAMPLES = [
    "Rust is a",
    "What is",
    "Hello",
    "The cat",
    "I love",
  ];

  const MAX_STEPS = 6;

  const runGeneration = useCallback(async () => {
    setSteps([]);
    setCurrentStep(-1);
    setIsRunning(true);
    stopRef.current = false;

    let currentInput = startText;
    const newSteps: Step[] = [];

    for (let i = 0; i < MAX_STEPS; i++) {
      if (stopRef.current) break;

      const tokens = tokenize(currentInput);
      const predictions = getPredictions(tokens);
      const winner = predictions[0]?.token || ".";

      const step: Step = {
        input: currentInput,
        tokens: tokens.map((t) => ({ text: t.text, id: t.id })),
        predictions: predictions.slice(0, 5),
        winner,
      };

      newSteps.push(step);
      setSteps([...newSteps]);
      setCurrentStep(i);

      // Wait so user can see each step
      await new Promise((r) => setTimeout(r, 1200));

      if (stopRef.current) break;

      // Append winner to input for next round
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
    setSteps([]);
    setCurrentStep(-1);
  };

  // Build the generated text so far
  const generatedText =
    steps.length > 0
      ? startText + " " + steps.map((s) => s.winner).join(" ")
      : startText;

  return (
    <div>
      <p className="text-slate-400 text-sm mb-4">
        An LLM generates text <strong className="text-slate-200">one token at a time</strong>.
        It runs the entire forward pass (tokenize → embed → 32 layers → predict) just to get
        ONE word. Then it appends that word, and does the whole thing again.
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

      {/* Generated text display */}
      <div className="mb-6 p-4 bg-slate-800/50 border border-slate-700 rounded-lg">
        <div className="flex gap-4 text-[10px] mb-2">
          <span className="text-slate-500">
            <span className="underline decoration-dashed decoration-slate-600 underline-offset-2">dashed underline</span> = your input (you typed this)
          </span>
          {steps.length > 0 && (
            <span className="text-emerald-500">
              green text = model generated (one word at a time)
            </span>
          )}
        </div>
        <div className="font-mono text-base">
          <span className="text-slate-400 underline decoration-slate-600 decoration-dashed underline-offset-4"
                title="This is YOUR input — the model did not generate this part"
          >{startText}</span>
          {steps.map((step, i) => (
            <span key={i}>
              {" "}
              <span
                className={`transition-all duration-300 ${
                  i === currentStep && isRunning
                    ? "text-emerald-400 bg-emerald-500/10 px-1 rounded"
                    : "text-emerald-300"
                }`}
              >
                {step.winner}
              </span>
            </span>
          ))}
          {isRunning && (
            <span className="inline-block w-2 h-4 bg-emerald-400 ml-0.5 animate-pulse" />
          )}
        </div>
      </div>

      {/* Step-by-step breakdown */}
      {steps.length > 0 && (
        <div className="space-y-3">
          <div className="text-xs text-slate-400 font-medium">
            Each pass through the model:
          </div>

          {steps.map((step, i) => (
            <div
              key={i}
              className={`p-3 rounded-lg border transition-all duration-500 ${
                i === currentStep && isRunning
                  ? "bg-slate-800/80 border-emerald-500/30 scale-[1.01]"
                  : i <= currentStep
                  ? "bg-slate-800/30 border-slate-700"
                  : "bg-slate-800/10 border-slate-800 opacity-50"
              }`}
            >
              {/* Pass number */}
              <div className="flex items-center gap-3 mb-2">
                <span
                  className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold shrink-0 ${
                    i === currentStep && isRunning
                      ? "bg-emerald-500/20 border border-emerald-500/40 text-emerald-300"
                      : "bg-slate-700 border border-slate-600 text-slate-400"
                  }`}
                >
                  {i + 1}
                </span>

                {/* Input for this pass */}
                <div className="flex-1 min-w-0">
                  <div className="text-[10px] text-slate-500 mb-0.5">
                    Pass {i + 1} input:
                  </div>
                  <div className="font-mono text-xs text-slate-300 truncate">
                    "{step.input}"
                  </div>
                </div>
              </div>

              {/* Mini pipeline */}
              <div className="flex items-center gap-1.5 ml-10 mb-2 text-[10px]">
                <span className="text-blue-400">
                  {step.tokens.length} tokens
                </span>
                <span className="text-slate-600">→</span>
                <span className="text-amber-400">embed</span>
                <span className="text-slate-600">→</span>
                <span className="text-purple-400">32 layers</span>
                <span className="text-slate-600">→</span>
                <span className="text-slate-400">predict</span>
              </div>

              {/* Top predictions */}
              <div className="ml-10 flex items-center gap-2">
                {step.predictions.slice(0, 4).map((p, j) => (
                  <span
                    key={j}
                    className={`text-[10px] font-mono px-1.5 py-0.5 rounded ${
                      j === 0
                        ? "bg-emerald-500/15 text-emerald-300 border border-emerald-500/20"
                        : "text-slate-500"
                    }`}
                  >
                    "{p.token}" {(p.probability * 100).toFixed(0)}%
                  </span>
                ))}
                <span className="text-slate-600 text-[10px]">→</span>
                <span className="text-[10px] font-mono font-bold text-emerald-400">
                  picks "{step.winner}"
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Summary after completion */}
      {steps.length > 0 && !isRunning && (
        <div className="mt-4 p-3 bg-emerald-500/5 border border-emerald-500/15 rounded-lg text-xs text-emerald-300">
          <strong>Done.</strong> The model ran {steps.length} forward passes
          to generate {steps.length} tokens. Each pass read ALL the model weights
          from memory. For a 7B model, that's reading ~4 GB of data per token.
          {steps.length} tokens = {steps.length} × 4 GB = ~{steps.length * 4} GB of
          data read from memory just for this short response.
        </div>
      )}

      {/* Reset */}
      {steps.length > 0 && !isRunning && (
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
