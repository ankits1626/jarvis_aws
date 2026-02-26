import { useState, useMemo } from "react";
import { tokenize } from "./lib/mockTokenizer";
import type { Token } from "./lib/mockTokenizer";
import BigPicture from "./components/BigPicture";
import TokenizerPanel from "./components/TokenizerPanel";
import EmbeddingIntuition from "./components/EmbeddingIntuition";
import EmbeddingPanel from "./components/EmbeddingPanel";
import AttentionPanel from "./components/AttentionPanel";
import LayersPanel from "./components/LayersPanel";
import PredictionPanel from "./components/PredictionPanel";
import AutoregressiveDemo from "./components/AutoregressiveDemo";
import KVCacheDemo from "./components/KVCacheDemo";
import WhatIsAModel from "./components/WhatIsAModel";
import PipelineExplorer from "./components/PipelineExplorer";

const EXAMPLE_SENTENCES = [
  "What is Rust?",
  "Rust is a programming language",
  "Rust on the iron door needs cleaned",
  "Hello world!",
  "The cat is happy",
  "I love tokenization",
];

interface NavItem {
  id: string;
  label: string;
  description: string;
}

interface NavSection {
  part: string;
  title: string;
  color: string;
  items: NavItem[];
}

const NAV_SECTIONS: NavSection[] = [
  {
    part: "Part 1",
    title: "What IS a Model?",
    color: "amber",
    items: [
      { id: "whatismodel", label: "Weights & Training", description: "The foundation" },
    ],
  },
  {
    part: "Part 2",
    title: "How It Works",
    color: "blue",
    items: [
      { id: "bigpicture", label: "The Big Picture", description: "Start here" },
      { id: "pipeline", label: "Pipeline Explorer", description: "Model at each step" },
      { id: "tokenize", label: "1. Tokenize", description: "Split text" },
      { id: "intuition", label: "2. Intuition", description: "Build understanding" },
      { id: "embed", label: "3. Embed", description: "Number lookup" },
      { id: "attention", label: "4. Attention", description: "Words look at words" },
      { id: "layers", label: "5. Layers", description: "Refine through layers" },
      { id: "predict", label: "6. Predict", description: "Pick next token" },
      { id: "generate", label: "7. Generate", description: "One token at a time" },
      { id: "kvcache", label: "8. KV Cache", description: "Speed optimization" },
    ],
  },
  {
    part: "Part 3",
    title: "Running Locally",
    color: "emerald",
    items: [
      { id: "modelsizes", label: "Model Sizes", description: "Why size = RAM" },
      { id: "quantization", label: "Quantization", description: "Compress weights" },
      { id: "memory", label: "Memory & Hardware", description: "Where weights live" },
    ],
  },
];

const TITLES: Record<string, string> = {
  whatismodel: "What IS a Model?",
  bigpicture: "The Big Picture: Text In, Word Out",
  tokenize: "Step 1: Tokenization",
  intuition: "Step 2: What Are Embeddings, Really?",
  embed: "Step 3: Embedding Lookup",
  attention: "Step 4: Attention — Words Looking at Words",
  layers: "Step 5: Forward Pass (Through All Layers)",
  predict: "Step 6: Predict Next Token",
  generate: "Step 7: Autoregressive Generation",
  kvcache: "Step 8: KV Cache — Don't Redo Work",
  pipeline: "Pipeline Explorer — Model at Each Step",
  modelsizes: "Why Model Size = RAM Needed",
  quantization: "Quantization — Making Models Fit",
  memory: "Memory & Hardware",
};

const HIDE_INPUT_STEPS = ["whatismodel", "bigpicture", "pipeline", "intuition", "generate", "kvcache", "modelsizes", "quantization", "memory"];

const COLOR_MAP: Record<string, { active: string; hover: string; border: string; text: string; bg: string }> = {
  amber: {
    active: "bg-amber-500/15 border-amber-500/30 text-amber-200",
    hover: "hover:bg-amber-500/5 hover:border-amber-500/15",
    border: "border-amber-500/20",
    text: "text-amber-400",
    bg: "bg-amber-500/10",
  },
  blue: {
    active: "bg-blue-500/15 border-blue-500/30 text-blue-200",
    hover: "hover:bg-blue-500/5 hover:border-blue-500/15",
    border: "border-blue-500/20",
    text: "text-blue-400",
    bg: "bg-blue-500/10",
  },
  emerald: {
    active: "bg-emerald-500/15 border-emerald-500/30 text-emerald-200",
    hover: "hover:bg-emerald-500/5 hover:border-emerald-500/15",
    border: "border-emerald-500/20",
    text: "text-emerald-400",
    bg: "bg-emerald-500/10",
  },
};

function App() {
  const [input, setInput] = useState("What is Rust?");
  const [activeStep, setActiveStep] = useState("whatismodel");
  const [selectedTokenIndex, setSelectedTokenIndex] = useState<number | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const tokens: Token[] = useMemo(() => tokenize(input), [input]);

  const handleSelectToken = (index: number) => {
    setSelectedTokenIndex(index);
    setActiveStep("embed");
  };

  const handleInputChange = (value: string) => {
    setInput(value);
    setSelectedTokenIndex(null);
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-200 flex flex-col">
      {/* Header */}
      <header className="border-b border-slate-800 px-6 py-3 shrink-0">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-lg font-bold text-white">LLM Visualizer</h1>
            <p className="text-xs text-slate-500">
              Learn how language models work, step by step
            </p>
          </div>
          <button
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            className="lg:hidden px-3 py-1.5 text-xs bg-slate-800 border border-slate-700
                       rounded-lg text-slate-400 hover:text-white transition-colors cursor-pointer"
          >
            {sidebarCollapsed ? "Show Nav" : "Hide Nav"}
          </button>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav
          className={`${
            sidebarCollapsed ? "hidden" : "flex"
          } lg:flex flex-col w-64 shrink-0 border-r border-slate-800 bg-slate-950
            overflow-y-auto`}
        >
          <div className="p-3 space-y-4">
            {NAV_SECTIONS.map((section) => {
              const colors = COLOR_MAP[section.color];
              return (
                <div key={section.part}>
                  {/* Section header */}
                  <div className="flex items-center gap-2 mb-1.5 px-2">
                    <span
                      className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${colors.bg} ${colors.text}`}
                    >
                      {section.part}
                    </span>
                    <span className="text-xs font-medium text-slate-400">
                      {section.title}
                    </span>
                  </div>

                  {/* Section items */}
                  <div className="space-y-0.5">
                    {section.items.map((item) => {
                      const isActive = activeStep === item.id;
                      return (
                        <button
                          key={item.id}
                          onClick={() => {
                            setActiveStep(item.id);
                            setSidebarCollapsed(true);
                          }}
                          className={`w-full text-left px-3 py-2 rounded-lg border text-xs
                                     transition-all cursor-pointer ${
                            isActive
                              ? colors.active
                              : `border-transparent text-slate-400 ${colors.hover}`
                          }`}
                        >
                          <div className="font-medium">{item.label}</div>
                          <div className={`text-[10px] mt-0.5 ${isActive ? "opacity-70" : "text-slate-600"}`}>
                            {item.description}
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        </nav>

        {/* Main content */}
        <main className="flex-1 overflow-y-auto">
          <div className="max-w-4xl mx-auto px-6 py-6">
            {/* Input section — hidden on pages that have their own input */}
            {!HIDE_INPUT_STEPS.includes(activeStep) && (
              <div className="mb-6">
                <label className="text-sm text-slate-400 block mb-2">
                  Type a sentence (or pick an example):
                </label>
                <input
                  type="text"
                  value={input}
                  onChange={(e) => handleInputChange(e.target.value)}
                  className="w-full px-4 py-3 bg-slate-900 border border-slate-700 rounded-lg
                             text-white font-mono text-lg focus:outline-none focus:border-blue-500
                             transition-colors"
                  placeholder="Type something..."
                />
                <div className="flex flex-wrap gap-2 mt-3">
                  {EXAMPLE_SENTENCES.map((sentence) => (
                    <button
                      key={sentence}
                      onClick={() => handleInputChange(sentence)}
                      className={`text-xs px-3 py-1.5 rounded-full border transition-colors cursor-pointer ${
                        input === sentence
                          ? "bg-blue-500/20 border-blue-400 text-blue-300"
                          : "bg-slate-800 border-slate-700 text-slate-400 hover:border-slate-500"
                      }`}
                    >
                      {sentence}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Active panel */}
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 min-h-[400px]">
              <h2 className="text-lg font-semibold text-white mb-4">
                {TITLES[activeStep] || activeStep}
              </h2>

              {activeStep === "whatismodel" && <WhatIsAModel />}
              {activeStep === "bigpicture" && <BigPicture />}
              {activeStep === "pipeline" && <PipelineExplorer />}
              {activeStep === "tokenize" && (
                <TokenizerPanel
                  tokens={tokens}
                  selectedTokenIndex={selectedTokenIndex}
                  onSelectToken={handleSelectToken}
                />
              )}
              {activeStep === "intuition" && <EmbeddingIntuition />}
              {activeStep === "embed" && (
                <EmbeddingPanel
                  tokens={tokens}
                  selectedTokenIndex={selectedTokenIndex}
                />
              )}
              {activeStep === "attention" && <AttentionPanel tokens={tokens} />}
              {activeStep === "layers" && (
                <LayersPanel
                  tokens={tokens}
                  selectedTokenIndex={selectedTokenIndex}
                />
              )}
              {activeStep === "predict" && <PredictionPanel tokens={tokens} />}
              {activeStep === "generate" && <AutoregressiveDemo />}
              {activeStep === "kvcache" && <KVCacheDemo />}

              {/* Part 3 placeholders */}
              {activeStep === "modelsizes" && (
                <div className="text-slate-500 text-sm italic">
                  Coming soon — why 7B means 14 GB, and why it all needs to fit in RAM.
                </div>
              )}
              {activeStep === "quantization" && (
                <div className="text-slate-500 text-sm italic">
                  Coming soon — how to shrink a 14 GB model to 4 GB by rounding numbers.
                </div>
              )}
              {activeStep === "memory" && (
                <div className="text-slate-500 text-sm italic">
                  Coming soon — unified memory, GPU vs CPU, and why Apple Silicon is special.
                </div>
              )}
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;
