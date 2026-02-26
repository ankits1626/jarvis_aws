// Mock model data to simulate embeddings, layer transformations, and predictions.
// All numbers are fake but structured to demonstrate the concepts.

import type { Token } from "./mockTokenizer";

// Deterministic pseudo-random based on a seed (so same token always gets same embedding)
function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 16807 + 0) % 2147483647;
    return (s / 2147483647) * 2 - 1; // Range: -1 to 1
  };
}

// Embedding dimension (real models use 4096, we use 8 for visualization)
export const EMBEDDING_DIM = 8;
export const NUM_LAYERS = 6; // Real models have 32, we use 6 to keep it simple

// Simple numbered labels — real dimensions have no human-readable names
export const DIMENSION_LABELS = [
  "dim 1",
  "dim 2",
  "dim 3",
  "dim 4",
  "dim 5",
  "dim 6",
  "dim 7",
  "dim 8",
];

// Generate a deterministic embedding for a token ID
export function getEmbedding(tokenId: number): number[] {
  const rng = seededRandom(tokenId * 7 + 13);
  return Array.from({ length: EMBEDDING_DIM }, () =>
    Math.round(rng() * 100) / 100
  );
}

// Simulate how layers transform embeddings based on context
// Each layer slightly shifts the numbers, and context words influence the shift
export function getLayerOutputs(
  tokens: Token[]
): { layer: number; embeddings: number[][] }[] {
  const layers: { layer: number; embeddings: number[][] }[] = [];

  // Start with raw embeddings
  let currentEmbeddings = tokens.map((t) => getEmbedding(t.id));
  layers.push({ layer: 0, embeddings: currentEmbeddings.map((e) => [...e]) });

  for (let l = 1; l <= NUM_LAYERS; l++) {
    const newEmbeddings: number[][] = [];

    for (let i = 0; i < tokens.length; i++) {
      const newEmb = [...currentEmbeddings[i]];

      // Simulate self-attention: each token is influenced by nearby tokens
      for (let j = 0; j < tokens.length; j++) {
        if (i === j) continue;
        // Influence decreases with distance and increases with layer depth
        const distance = Math.abs(i - j);
        const influence = (0.05 * l) / (distance + 1);

        for (let d = 0; d < EMBEDDING_DIM; d++) {
          newEmb[d] += currentEmbeddings[j][d] * influence;
        }
      }

      // Normalize and round for readability
      const maxVal = Math.max(...newEmb.map(Math.abs));
      const normalized = newEmb.map((v) =>
        Math.round((v / (maxVal || 1)) * 100) / 100
      );
      newEmbeddings.push(normalized);
    }

    currentEmbeddings = newEmbeddings;
    layers.push({
      layer: l,
      embeddings: newEmbeddings.map((e) => [...e]),
    });
  }

  return layers;
}

// Simulate next-token prediction probabilities
// Returns top candidates with probabilities
export type Prediction = {
  token: string;
  probability: number;
}

const CONTEXTUAL_PREDICTIONS: Record<string, Prediction[]> = {
  "what is": [
    { token: "the", probability: 0.15 },
    { token: "a", probability: 0.12 },
    { token: "it", probability: 0.08 },
    { token: "this", probability: 0.07 },
    { token: "your", probability: 0.06 },
    { token: "that", probability: 0.05 },
    { token: "Rust", probability: 0.04 },
    { token: "an", probability: 0.04 },
    { token: "going", probability: 0.03 },
    { token: "love", probability: 0.02 },
  ],
  "what is rust": [
    { token: "?", probability: 0.25 },
    { token: "programming", probability: 0.12 },
    { token: "used", probability: 0.10 },
    { token: "and", probability: 0.08 },
    { token: "language", probability: 0.06 },
    { token: "good", probability: 0.05 },
    { token: "really", probability: 0.04 },
    { token: ",", probability: 0.03 },
    { token: "exactly", probability: 0.03 },
    { token: "all", probability: 0.02 },
  ],
  "rust is a": [
    { token: "programming", probability: 0.35 },
    { token: "systems", probability: 0.15 },
    { token: "language", probability: 0.10 },
    { token: "fast", probability: 0.06 },
    { token: "new", probability: 0.05 },
    { token: "good", probability: 0.04 },
    { token: "great", probability: 0.03 },
    { token: "modern", probability: 0.03 },
    { token: "memory", probability: 0.02 },
    { token: "safe", probability: 0.02 },
  ],
  "rust is a programming": [
    { token: "language", probability: 0.82 },
    { token: "tool", probability: 0.05 },
    { token: "framework", probability: 0.03 },
    { token: "environment", probability: 0.02 },
    { token: "system", probability: 0.01 },
    { token: "paradigm", probability: 0.01 },
    { token: "concept", probability: 0.005 },
    { token: "model", probability: 0.005 },
    { token: "approach", probability: 0.003 },
    { token: "method", probability: 0.002 },
  ],
  "hello": [
    { token: "!", probability: 0.30 },
    { token: ",", probability: 0.20 },
    { token: "world", probability: 0.15 },
    { token: "there", probability: 0.10 },
    { token: ".", probability: 0.05 },
    { token: "how", probability: 0.04 },
    { token: "everyone", probability: 0.03 },
    { token: "and", probability: 0.02 },
    { token: "I", probability: 0.02 },
    { token: "my", probability: 0.01 },
  ],
  "hello world": [
    { token: "!", probability: 0.45 },
    { token: ".", probability: 0.15 },
    { token: ",", probability: 0.08 },
    { token: "program", probability: 0.05 },
    { token: "in", probability: 0.04 },
    { token: "is", probability: 0.03 },
    { token: "example", probability: 0.03 },
    { token: "app", probability: 0.02 },
    { token: "application", probability: 0.02 },
    { token: "code", probability: 0.01 },
  ],
  "the cat": [
    { token: "is", probability: 0.25 },
    { token: "sat", probability: 0.12 },
    { token: "was", probability: 0.10 },
    { token: "and", probability: 0.08 },
    { token: "in", probability: 0.06 },
    { token: "ran", probability: 0.05 },
    { token: "has", probability: 0.04 },
    { token: "on", probability: 0.03 },
    { token: "looks", probability: 0.03 },
    { token: "jumped", probability: 0.02 },
  ],
  "i love": [
    { token: "the", probability: 0.15 },
    { token: "you", probability: 0.14 },
    { token: "it", probability: 0.10 },
    { token: "this", probability: 0.08 },
    { token: "my", probability: 0.06 },
    { token: "how", probability: 0.05 },
    { token: "that", probability: 0.05 },
    { token: "programming", probability: 0.04 },
    { token: "Rust", probability: 0.03 },
    { token: "to", probability: 0.03 },
  ],
  "rust on the iron door": [
    { token: "needs", probability: 0.20 },
    { token: "is", probability: 0.15 },
    { token: "was", probability: 0.10 },
    { token: "has", probability: 0.08 },
    { token: "should", probability: 0.06 },
    { token: "must", probability: 0.05 },
    { token: "can", probability: 0.04 },
    { token: ".", probability: 0.04 },
    { token: "looks", probability: 0.03 },
    { token: "had", probability: 0.03 },
  ],
};

export function getPredictions(tokens: Token[]): Prediction[] {
  // Build a lowercase key from the last few tokens
  const text = tokens
    .map((t) => t.text)
    .join(" ")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();

  // Try matching from longest to shortest
  for (let len = tokens.length; len >= 1; len--) {
    const key = tokens
      .slice(-len)
      .map((t) => t.text)
      .join(" ")
      .toLowerCase()
      .replace(/\s+/g, " ")
      .trim();
    if (CONTEXTUAL_PREDICTIONS[key]) {
      return CONTEXTUAL_PREDICTIONS[key];
    }
  }

  // Default predictions if no match
  const rng = seededRandom(text.length * 31 + text.charCodeAt(0));
  const defaultWords = [
    "the",
    "is",
    "a",
    "and",
    "to",
    "in",
    "of",
    "that",
    "it",
    "for",
  ];
  let remaining = 1.0;
  return defaultWords.map((word, i) => {
    const prob =
      i < 9 ? Math.abs(rng()) * remaining * 0.5 : remaining;
    remaining -= prob;
    return { token: word, probability: Math.round(prob * 100) / 100 };
  });
}
