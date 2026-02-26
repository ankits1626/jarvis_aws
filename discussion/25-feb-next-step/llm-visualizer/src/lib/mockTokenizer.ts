// A small fake vocabulary to simulate how real tokenizers work.
// Real models have 32,000-128,000 entries. We have ~100 to keep it understandable.

const VOCABULARY: Record<string, number> = {
  // Common words (get their own token)
  "the": 279,
  "a": 64,
  "an": 276,
  "is": 374,
  "are": 527,
  "was": 665,
  "in": 304,
  "on": 389,
  "to": 311,
  "of": 315,
  "and": 323,
  "for": 369,
  "it": 433,
  "that": 429,
  "this": 576,
  "with": 449,
  "not": 539,
  "but": 719,
  "from": 505,
  "or": 477,
  "be": 387,
  "have": 617,
  "has": 706,
  "had": 952,
  "do": 656,
  "does": 1587,
  "will": 690,
  "can": 649,
  "I": 40,
  "you": 366,
  "he": 568,
  "she": 7091,
  "we": 584,
  "they": 814,
  "my": 856,

  // Question words
  "what": 3923,
  "What": 3923,
  "how": 5765,
  "How": 5765,
  "why": 8823,
  "Why": 8823,
  "where": 2940,
  "when": 3228,
  "who": 6516,

  // Tech / content words
  "Rust": 56461,
  "rust": 56461,
  "Python": 31380,
  "python": 31380,
  "programming": 15840,
  "language": 11513,
  "code": 1889,
  "model": 2746,
  "token": 4037,
  "computer": 17642,
  "data": 1473,
  "learning": 6975,
  "machine": 5765,
  "neural": 30828,
  "network": 4992,
  "AI": 15836,
  "hello": 15339,
  "Hello": 15339,
  "world": 1917,

  // Everyday words
  "cat": 4937,
  "dog": 5765,
  "happy": 9796,
  "sad": 12233,
  "big": 3016,
  "small": 2831,
  "good": 1695,
  "bad": 3958,
  "new": 943,
  "old": 1866,
  "fast": 4428,
  "slow": 6435,
  "door": 10694,
  "iron": 18534,
  "clean": 11547,
  "needs": 4460,
  "cleaned": 28208,

  // Verbs
  "run": 3220,
  "runs": 8640,
  "write": 5765,
  "read": 1493,
  "think": 8774,
  "like": 1093,
  "love": 8834,
  "make": 1652,
  "go": 733,
  "see": 1518,
  "know": 1440,
  "get": 636,

  // Articles / prepositions (these are very common, low IDs)
  " ": 220,

  // Punctuation
  ".": 13,
  ",": 11,
  "!": 0,
  "?": 30,
  ":": 25,
  ";": 26,
  "'": 6,
  '"': 1,
  "(": 7,
  ")": 8,
  "-": 12,
};

// Subword pieces for words NOT in vocabulary
const SUBWORD_PIECES: Record<string, string[]> = {
  "tokenization": ["token", "ization"],
  "Tokenization": ["Token", "ization"],
  "defenestration": ["def", "en", "est", "ration"],
  "ineffably": ["ine", "ff", "ably"],
  "transformer": ["trans", "former"],
  "embeddings": ["embed", "dings"],
  "embedding": ["embed", "ding"],
  "JavaScript": ["Java", "Script"],
  "javascript": ["java", "script"],
  "TypeScript": ["Type", "Script"],
  "kubernetes": ["kube", "rne", "tes"],
  "authentication": ["auth", "enti", "cation"],
  "beautiful": ["beaut", "iful"],
  "understand": ["under", "stand"],
  "understanding": ["under", "stand", "ing"],
  "intelligence": ["intel", "lig", "ence"],
  "artificial": ["art", "ific", "ial"],
};

// Assign IDs to subword pieces
const SUBWORD_IDS: Record<string, number> = {
  "token": 4037,
  "Token": 4037,
  "ization": 2065,
  "def": 1316,
  "en": 268,
  "est": 478,
  "ration": 7761,
  "ine": 1130,
  "ff": 544,
  "ably": 2448,
  "trans": 3286,
  "former": 13234,
  "embed": 11641,
  "dings": 67249,
  "ding": 5765,
  "Java": 21890,
  "java": 21890,
  "Script": 7912,
  "script": 7912,
  "Type": 941,
  "kube": 63541,
  "rne": 42134,
  "tes": 2423,
  "auth": 5765,
  "enti": 22068,
  "cation": 29715,
  "beaut": 1461,
  "iful": 9128,
  "under": 8154,
  "stand": 9761,
  "ing": 278,
  "intel": 30219,
  "lig": 22593,
  "ence": 768,
  "art": 1802,
  "ific": 28340,
  "ial": 532,
};

export type Token = {
  text: string;
  id: number;
  isSubword: boolean;
  originalWord?: string;
}

export function tokenize(input: string): Token[] {
  if (!input.trim()) return [];

  const tokens: Token[] = [];

  // Split by spaces but keep track of spacing
  const words = input.match(/\S+|\s+/g) || [];

  for (const word of words) {
    // Skip pure whitespace
    if (/^\s+$/.test(word)) continue;

    // Separate punctuation from the word
    const parts = word.match(/[a-zA-Z']+|[^a-zA-Z'\s]/g) || [word];

    for (const part of parts) {
      // Check if it's punctuation
      if (VOCABULARY[part] !== undefined && part.length === 1 && /[^a-zA-Z]/.test(part)) {
        tokens.push({ text: part, id: VOCABULARY[part], isSubword: false });
        continue;
      }

      // Check if whole word is in vocabulary
      if (VOCABULARY[part] !== undefined) {
        tokens.push({ text: part, id: VOCABULARY[part], isSubword: false });
        continue;
      }

      // Check lowercase version
      if (VOCABULARY[part.toLowerCase()] !== undefined) {
        tokens.push({ text: part, id: VOCABULARY[part.toLowerCase()], isSubword: false });
        continue;
      }

      // Check if word has subword pieces defined
      const subwords = SUBWORD_PIECES[part] || SUBWORD_PIECES[part.toLowerCase()];
      if (subwords) {
        subwords.forEach((sw) => {
          const id = SUBWORD_IDS[sw] || Math.floor(Math.random() * 50000) + 10000;
          tokens.push({ text: sw, id, isSubword: true, originalWord: part });
        });
        continue;
      }

      // Unknown word: split into character-level pieces (simulating BPE fallback)
      const chars = part.split("");
      chars.forEach((ch) => {
        const id = VOCABULARY[ch] || ch.charCodeAt(0) + 1000;
        tokens.push({ text: ch, id, isSubword: true, originalWord: part });
      });
    }
  }

  return tokens;
}

export function getVocabularySize(): number {
  return 32000; // Simulating Llama 2's vocab size
}
