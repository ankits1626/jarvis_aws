import type { Guide } from './types'

export const reactTSGuide: Guide = {
  id: 'guide-react',
  title: 'React + TypeScript 101',
  subtitle: 'Build interactive UIs by describing what you want — the framework behind Jarvis\'s frontend',
  color: 'blue',
  icon: '⚛️',
  sections: [
    // ── 0. What Are We Building? ──
    {
      id: 'big-picture',
      title: 'What Are We Building?',
      content: [
        {
          type: 'text',
          body: 'In Tauri 101 you built a frontend with plain HTML and JavaScript — writing document.querySelector, manually updating text, wiring up event listeners. That works for 4 buttons, but Jarvis has dozens of screens, forms, lists, modals, and live-updating data. Managing all that with plain JS becomes a tangled mess. React solves this.',
        },
        {
          type: 'text',
          body: 'We\'re going to build a "Gem Viewer" — a small React app that shows a list of gems, lets you add new ones, search through them, and tag them. Along the way you\'ll learn every React + TypeScript concept that Jarvis uses.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'setup', label: '1. Setup', icon: '🔧' },
            { id: 'comp', label: '2. Components', icon: '🧱' },
            { id: 'jsx', label: '3. JSX', icon: '📝' },
            { id: 'props', label: '4. Props', icon: '📦' },
            { id: 'state', label: '5. State', icon: '🔄' },
            { id: 'events', label: '6. Events', icon: '👆' },
            { id: 'lists', label: '7. Lists', icon: '📋' },
            { id: 'ts', label: '8. TypeScript', icon: '🛡️' },
            { id: 'hooks', label: '9. Hooks', icon: '🪝' },
          ],
          connections: [
            { from: 'setup', to: 'comp' },
            { from: 'comp', to: 'jsx' },
            { from: 'jsx', to: 'props' },
            { from: 'props', to: 'state' },
            { from: 'state', to: 'events' },
            { from: 'events', to: 'lists' },
            { from: 'lists', to: 'ts' },
            { from: 'ts', to: 'hooks' },
          ],
        },
      ],
    },

    // ── 1. The Core Idea ──
    {
      id: 'core-idea',
      title: 'Step 1: The Core Idea — UI = f(data)',
      content: [
        {
          type: 'text',
          body: 'With plain JavaScript, you tell the browser STEP BY STEP what to change: "find this element, change its text, add a class, remove that child." It\'s like giving someone driving directions turn by turn.',
        },
        {
          type: 'text',
          body: 'React flips this. You DESCRIBE what the screen should look like given some data, and React figures out what needs to change. It\'s like giving someone an address — they use GPS to figure out the route. You say "show 3 gems" and React handles adding/removing the right HTML elements.',
        },
        {
          type: 'comparison',
          leftLabel: 'Plain JavaScript',
          rightLabel: 'React',
          rows: [
            { label: 'Approach', left: 'Tell the browser what to DO', right: 'Describe what the screen should LOOK LIKE' },
            { label: 'Update a list', left: 'Find the <ul>, create <li>, append it', right: 'Change the data array, React updates the DOM' },
            { label: 'Show/hide', left: 'element.style.display = "none"', right: 'Just don\'t include it in the output' },
            { label: 'Analogy', left: 'Turn-by-turn directions', right: 'Give the destination, GPS handles the route' },
            { label: 'Bug risk', left: 'Easy to forget an update', right: 'Screen always matches the data' },
          ],
        },
        {
          type: 'code',
          language: 'javascript',
          code: '// PLAIN JS: Step-by-step instructions\nconst li = document.createElement("li");\nli.textContent = "New gem";\nli.className = "gem-item";\ndocument.querySelector("#gem-list").appendChild(li);\n// What if the list doesn\'t exist yet? What if we need to\n// also update the count? Easy to forget something.',
          caption: 'Imperative: you manage every DOM change yourself',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// REACT: Describe what the screen should look like\nfunction GemList({ gems }) {\n  return (\n    <div>\n      <p>{gems.length} gems</p>\n      <ul>\n        {gems.map(gem => <li key={gem.id}>{gem.title}</li>)}\n      </ul>\n    </div>\n  );\n}\n// The count and list ALWAYS match. Add a gem to the\n// array → React updates both automatically.',
          caption: 'Declarative: describe the result, React handles the DOM changes',
        },
        {
          type: 'quiz',
          question: 'You have a list of 5 gems displayed. The user adds a 6th gem. In React, what do you do?',
          options: [
            'Find the <ul> element and append a new <li>',
            'Add the gem to the data array — React re-renders the list automatically',
            'Reload the entire page',
            'Manually update the count and the list separately',
          ],
          correctIndex: 1,
          explanation: 'In React, you just update the data. React compares what the screen SHOULD look like (6 gems) with what it currently shows (5 gems), and makes the minimal DOM changes needed. You never touch the DOM directly.',
        },
      ],
    },

    // ── 2. Setting Up ──
    {
      id: 'setup',
      title: 'Step 2: Setting Up Your React Project',
      content: [
        {
          type: 'text',
          body: 'We\'ll use Vite (pronounced "veet") — a fast build tool that sets up React + TypeScript for you. It\'s what the Jarvis visualizer uses, and it\'s the modern standard for React projects.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Create a new React + TypeScript project\nnpm create vite@latest gem-viewer -- --template react-ts\n\n# Go into it and install dependencies\ncd gem-viewer\nnpm install\n\n# Start the dev server (with hot-reload!)\nnpm run dev',
          caption: 'Open http://localhost:5173 in your browser. You should see a React starter page.',
        },
        {
          type: 'code',
          language: 'bash',
          code: 'gem-viewer/\n├── src/\n│   ├── App.tsx          # Your main component (start here)\n│   ├── main.tsx         # Entry point (mounts React)\n│   └── App.css          # Styles\n├── index.html           # The HTML shell\n├── package.json         # Dependencies\n├── tsconfig.json        # TypeScript settings\n└── vite.config.ts       # Build tool config',
          caption: 'You\'ll spend 99% of your time in src/. App.tsx is where the action starts.',
        },
        {
          type: 'text',
          body: 'Open src/App.tsx — delete everything in it. We\'re starting from scratch. Replace it with this:',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// src/App.tsx\nfunction App() {\n  return (\n    <div>\n      <h1>Gem Viewer</h1>\n      <p>My gem collection</p>\n    </div>\n  )\n}\n\nexport default App',
          caption: 'Your first React component. Save the file — the browser updates instantly (hot-reload).',
        },
        {
          type: 'text',
          body: 'Wait — is that HTML inside JavaScript?! Yes. That weird mix of HTML-in-JS is called JSX, and it\'s the key idea behind React. Let\'s understand it.',
        },
      ],
    },

    // ── 3. Components — Building Blocks ──
    {
      id: 'components',
      title: 'Step 3: Components — LEGO Blocks for Your UI',
      content: [
        {
          type: 'text',
          body: 'Think about LEGO. You don\'t build a castle from one giant piece — you snap together small, reusable blocks. React works the same way. A "component" is a small, reusable piece of UI. A button is a component. A search bar is a component. A gem card is a component. Your whole app is components inside components.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'app', label: 'App', icon: '📱' },
            { id: 'header', label: 'Header', icon: '🏷️' },
            { id: 'search', label: 'SearchBar', icon: '🔍' },
            { id: 'list', label: 'GemList', icon: '📋' },
            { id: 'card', label: 'GemCard (x3)', icon: '💎' },
          ],
          connections: [
            { from: 'app', to: 'header' },
            { from: 'app', to: 'search' },
            { from: 'app', to: 'list' },
            { from: 'list', to: 'card' },
          ],
        },
        {
          type: 'text',
          body: 'A component is just a function that returns what should appear on screen. That\'s it. The function name IS the component name, and it must start with a capital letter.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// A component is just a function that returns UI\nfunction Header() {\n  return (\n    <header>\n      <h1>Gem Viewer</h1>\n      <p>Your knowledge collection</p>\n    </header>\n  )\n}\n\nfunction SearchBar() {\n  return (\n    <div>\n      <input placeholder="Search gems..." />\n    </div>\n  )\n}\n\n// Use them like custom HTML tags!\nfunction App() {\n  return (\n    <div>\n      <Header />\n      <SearchBar />\n    </div>\n  )\n}\n\nexport default App',
          caption: 'Components snap together like LEGO. <Header /> and <SearchBar /> are your custom building blocks.',
        },
        {
          type: 'concept-card',
          term: 'Component',
          explanation: 'A function that returns a piece of UI (JSX). Think of it as a custom HTML tag that you define. Components must start with a capital letter: Header, GemCard, SearchBar.',
          example: 'function GemCard() {\n  return <div className="card">A gem!</div>\n}',
        },
        {
          type: 'concept-card',
          term: 'JSX',
          explanation: 'The HTML-like syntax inside JavaScript. It LOOKS like HTML but it\'s actually JavaScript that creates elements. Key differences: use className instead of class, and you can embed JavaScript with {curly braces}.',
          example: 'return (\n  <div className="card">\n    <p>{2 + 2}</p>  {/* Shows: 4 */}\n  </div>\n)',
        },
        {
          type: 'quiz',
          question: 'Why do we split the UI into many small components instead of writing one big function?',
          options: [
            'React requires it — you can\'t have one big component',
            'Same reason you use functions instead of one giant script: reusability, readability, easier debugging',
            'Small components are faster',
            'TypeScript requires separate components',
          ],
          correctIndex: 1,
          explanation: 'A SearchBar component can be reused on different pages. A GemCard component can be debugged in isolation. When something breaks, you know exactly which piece to look at. Same principle as functions in any language.',
        },
      ],
    },

    // ── 4. Props — Passing Data Down ──
    {
      id: 'props',
      title: 'Step 4: Props — Giving Data to Components',
      content: [
        {
          type: 'text',
          body: 'A GemCard that always shows the same text is useless. We need to GIVE it data: "here\'s the title, here\'s the content, here are the tags." Props (short for "properties") are how you pass data from a parent component to a child. Think of props as the order slip you hand to the chef — "make THIS dish with THESE ingredients."',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// The component receives props as a parameter\nfunction GemCard({ title, content, tags }) {\n  return (\n    <div className="card">\n      <h3>{title}</h3>\n      <p>{content}</p>\n      <div>\n        {tags.map(tag => (\n          <span key={tag} className="tag">#{tag}</span>\n        ))}\n      </div>\n    </div>\n  )\n}\n\n// Pass data when using the component\nfunction App() {\n  return (\n    <div>\n      <GemCard\n        title="Rust Ownership"\n        content="Every value has exactly one owner"\n        tags={["rust", "learning"]}\n      />\n      <GemCard\n        title="Tauri Commands"\n        content="Use invoke() to call Rust from JS"\n        tags={["tauri", "ipc"]}\n      />\n    </div>\n  )\n}',
          caption: 'Same component, different data. Two GemCards showing different gems.',
        },
        {
          type: 'text',
          body: 'Notice the {curly braces}? Inside JSX, anything in {} is JavaScript. So {title} displays the value of the title variable, and {tags.map(...)} runs JavaScript to create elements for each tag. Text like "Rust Ownership" is just a string — no braces needed because it\'s not a variable.',
        },
        {
          type: 'concept-card',
          term: 'Props',
          explanation: 'Data passed from parent to child. The parent decides WHAT to show, the child decides HOW to show it. Props flow ONE way: down. A child can\'t change its own props.',
          example: '<GemCard title="My Gem" tags={["work"]} />\n// GemCard receives { title, tags }',
        },
        {
          type: 'concept-card',
          term: '{curly braces} in JSX',
          explanation: '"Switch to JavaScript mode." Anything inside {} is evaluated as JavaScript. Use it to display variables, do math, call functions, or write conditions.',
          example: '<p>{user.name}</p>        {/* variable */}\n<p>{2 + 2}</p>            {/* math: 4 */}\n<p>{isNew ? "New!" : ""}</p> {/* condition */}',
        },
        {
          type: 'interactive-code',
          language: 'typescript',
          starterCode: '// Create a TagBadge component that receives\n// a "label" prop and displays it as #label\n// Then use it: <TagBadge label="rust" />\n\nfunction TagBadge(/* what goes here? */) {\n  return (\n    // what goes here?\n  )\n}',
          solution: 'function TagBadge({ label }: { label: string }) {\n  return (\n    <span className="tag">#{label}</span>\n  )\n}',
          hint: 'The function takes { label } as a parameter (destructured from props). Return a <span> that shows #{label}.',
          validator: (input: string) => {
            return input.includes('TagBadge') && input.includes('label') && input.includes('#')
          },
        },
      ],
    },

    // ── 5. State — Making Things Interactive ──
    {
      id: 'state',
      title: 'Step 5: State — Memory That Triggers Updates',
      content: [
        {
          type: 'text',
          body: 'Props are data that comes from OUTSIDE (the parent passes it). But what about data that changes INSIDE a component? Like a counter, a search query being typed, or whether a dropdown is open. This is "state" — the component\'s own memory. When state changes, React automatically re-renders the component.',
        },
        {
          type: 'text',
          body: 'Think of a light switch. It has a state: on or off. When you flip it (change the state), the light changes (the UI updates). You don\'t manually rewire the light every time — the switch TRIGGERS the change. That\'s what React state does.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// src/App.tsx\nimport { useState } from "react"\n\nfunction App() {\n  // useState returns: [current value, function to change it]\n  const [count, setCount] = useState(0)\n  //       ^         ^                 ^\n  //    the value  the updater    starting value\n\n  return (\n    <div>\n      <p>You clicked {count} times</p>\n      <button onClick={() => setCount(count + 1)}>\n        Click me\n      </button>\n    </div>\n  )\n}\n// Every time you click, setCount updates the value,\n// React re-renders, and the <p> shows the new count.\n// You never touch the DOM.',
          caption: 'useState is the most fundamental React hook. The UI always reflects the current state.',
        },
        {
          type: 'text',
          body: 'Let\'s build something real — a search filter for our gems:',
        },
        {
          type: 'code',
          language: 'typescript',
          code: 'import { useState } from "react"\n\nconst ALL_GEMS = [\n  { id: 1, title: "Rust Ownership", tags: ["rust"] },\n  { id: 2, title: "Tauri Commands", tags: ["tauri"] },\n  { id: 3, title: "React State", tags: ["react"] },\n]\n\nfunction App() {\n  const [query, setQuery] = useState("")\n\n  // Filter gems based on what\'s typed\n  const filtered = ALL_GEMS.filter(gem =>\n    gem.title.toLowerCase().includes(query.toLowerCase())\n  )\n\n  return (\n    <div>\n      <input\n        placeholder="Search gems..."\n        value={query}\n        onChange={(e) => setQuery(e.target.value)}\n      />\n      <p>{filtered.length} results</p>\n      {filtered.map(gem => (\n        <div key={gem.id}>{gem.title}</div>\n      ))}\n    </div>\n  )\n}',
          caption: 'Type "rust" → setQuery updates → React re-renders → only matching gems show. The whole UI stays in sync automatically.',
        },
        {
          type: 'concept-card',
          term: 'useState(initialValue)',
          explanation: 'Creates a piece of state. Returns [value, setter]. Call the setter to update the value and trigger a re-render. Never modify the value directly — always use the setter.',
          example: 'const [name, setName] = useState("")\nconst [items, setItems] = useState([])\nconst [isOpen, setIsOpen] = useState(false)',
        },
        {
          type: 'concept-card',
          term: 'Re-render',
          explanation: 'When state changes, React calls your component function again with the new state. It compares the old and new output, and updates only the DOM elements that actually changed. This is fast — React doesn\'t recreate everything.',
        },
        {
          type: 'quiz',
          question: 'Why can\'t you just write `count = count + 1` instead of `setCount(count + 1)`?',
          options: [
            'JavaScript doesn\'t allow reassignment',
            'Changing a variable directly doesn\'t tell React to re-render — the screen wouldn\'t update',
            'setCount is faster',
            'TypeScript prevents direct assignment',
          ],
          correctIndex: 1,
          explanation: 'React doesn\'t watch your variables. It only knows to update the screen when you call the setter (setCount). If you just change the variable, React has no idea anything happened — the screen stays frozen on the old value.',
        },
      ],
    },

    // ── 6. Events — Responding to the User ──
    {
      id: 'events',
      title: 'Step 6: Events — Responding to Clicks, Types, and Submits',
      content: [
        {
          type: 'text',
          body: 'In plain HTML you write onclick="doSomething()". In React you write onClick={doSomething} — camelCase and a function reference (not a string). Events are how your UI responds to the user: clicks, typing, form submissions, keyboard shortcuts.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: 'function GemForm() {\n  const [title, setTitle] = useState("")\n  const [content, setContent] = useState("")\n\n  function handleSubmit(e: React.FormEvent) {\n    e.preventDefault()  // Don\'t reload the page!\n    console.log("New gem:", title, content)\n    setTitle("")   // Clear the form\n    setContent("")\n  }\n\n  return (\n    <form onSubmit={handleSubmit}>\n      <input\n        value={title}\n        onChange={(e) => setTitle(e.target.value)}\n        placeholder="Gem title"\n      />\n      <textarea\n        value={content}\n        onChange={(e) => setContent(e.target.value)}\n        placeholder="Gem content"\n      />\n      <button type="submit">Add Gem</button>\n    </form>\n  )\n}',
          caption: 'A controlled form: React owns the input values. onChange updates state, state updates the input.',
        },
        {
          type: 'text',
          body: 'Notice the pattern: the input\'s VALUE comes from state, and onChange UPDATES the state. This is called a "controlled input" — React is the single source of truth. The input always shows exactly what\'s in state. This prevents the input and your data from getting out of sync.',
        },
        {
          type: 'comparison',
          leftLabel: 'Plain HTML',
          rightLabel: 'React',
          rows: [
            { label: 'Click', left: 'onclick="fn()"', right: 'onClick={fn}' },
            { label: 'Typing', left: 'oninput="fn()"', right: 'onChange={fn}' },
            { label: 'Submit', left: 'onsubmit="fn()"', right: 'onSubmit={fn}' },
            { label: 'Naming', left: 'lowercase', right: 'camelCase' },
            { label: 'Value', left: 'String of code', right: 'Function reference' },
          ],
        },
        {
          type: 'interactive-code',
          language: 'typescript',
          starterCode: '// Build a toggle button that switches between\n// "Recording" and "Stopped"\n// Hint: use useState with a boolean\nimport { useState } from "react"\n\nfunction RecordButton() {\n  // your code here\n}',
          solution: 'import { useState } from "react"\n\nfunction RecordButton() {\n  const [isRecording, setIsRecording] = useState(false)\n\n  return (\n    <button onClick={() => setIsRecording(!isRecording)}>\n      {isRecording ? "Recording..." : "Start Recording"}\n    </button>\n  )\n}',
          hint: 'Create a boolean state: const [isRecording, setIsRecording] = useState(false). Toggle it onClick. Show different text with a ternary: {isRecording ? "Recording..." : "Start"}.',
          validator: (input: string) => {
            return input.includes('useState') && input.includes('onClick') && input.includes('isRecording')
          },
        },
      ],
    },

    // ── 7. Lists — Rendering Many Items ──
    {
      id: 'lists',
      title: 'Step 7: Lists — Displaying Many Gems',
      content: [
        {
          type: 'text',
          body: 'Our gem viewer needs to show a LIST of gems — not just one or two hardcoded ones. In React, you use .map() to turn an array of data into an array of components. Think of it as a factory conveyor belt: data goes in one side, UI pieces come out the other.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: 'const gems = [\n  { id: 1, title: "Rust Ownership", tags: ["rust"] },\n  { id: 2, title: "Tauri Commands", tags: ["tauri"] },\n  { id: 3, title: "React State", tags: ["react"] },\n]\n\nfunction GemList() {\n  return (\n    <ul>\n      {gems.map(gem => (\n        <li key={gem.id}>\n          <strong>{gem.title}</strong>\n          {gem.tags.map(tag => (\n            <span key={tag}> #{tag}</span>\n          ))}\n        </li>\n      ))}\n    </ul>\n  )\n}',
          caption: '.map() converts each data item into a JSX element. The key prop helps React track which items changed.',
        },
        {
          type: 'concept-card',
          term: 'key prop',
          explanation: 'A unique identifier for each item in a list. React uses it to track which items were added, removed, or moved. Without keys, React re-renders the entire list on every change (slow). With keys, it only updates what changed.',
          example: '{items.map(item => (\n  <div key={item.id}>  {/* unique! */}\n    {item.name}\n  </div>\n))}',
        },
        {
          type: 'text',
          body: 'Now let\'s combine everything — a list with an add form:',
        },
        {
          type: 'code',
          language: 'typescript',
          code: 'import { useState } from "react"\n\nfunction App() {\n  const [gems, setGems] = useState([\n    { id: 1, title: "Rust Ownership" },\n    { id: 2, title: "Tauri Commands" },\n  ])\n  const [newTitle, setNewTitle] = useState("")\n\n  function addGem() {\n    if (!newTitle.trim()) return\n    setGems([...gems, {\n      id: Date.now(),  // simple unique ID\n      title: newTitle,\n    }])\n    setNewTitle("")  // clear the input\n  }\n\n  return (\n    <div>\n      <h1>Gem Viewer ({gems.length})</h1>\n      <input\n        value={newTitle}\n        onChange={e => setNewTitle(e.target.value)}\n        placeholder="New gem title"\n      />\n      <button onClick={addGem}>Add</button>\n      <ul>\n        {gems.map(gem => (\n          <li key={gem.id}>{gem.title}</li>\n        ))}\n      </ul>\n    </div>\n  )\n}',
          caption: 'The full pattern: state holds the list, form adds to it, .map() renders it. Everything stays in sync.',
        },
        {
          type: 'text',
          body: 'Notice: we don\'t do gems.push(newGem). In React, you never mutate state directly. Instead, you create a NEW array with the spread operator: [...gems, newGem]. This tells React "something changed" and triggers a re-render. If you push() onto the existing array, React won\'t know anything happened.',
        },
        {
          type: 'quiz',
          question: 'Why do we write setGems([...gems, newGem]) instead of gems.push(newGem)?',
          options: [
            'push() is slower',
            'TypeScript doesn\'t allow push()',
            'React only re-renders when you call the setter with a NEW value — push() modifies the same array in place, so React sees no change',
            'Spread syntax is newer JavaScript',
          ],
          correctIndex: 2,
          explanation: 'React compares the old and new values. If you push() onto the same array, the reference hasn\'t changed — React thinks nothing happened. By creating a new array with [...gems, newGem], React sees a different reference and re-renders.',
        },
      ],
    },

    // ── 8. TypeScript — Safety for Your UI ──
    {
      id: 'typescript',
      title: 'Step 8: TypeScript — Catching Mistakes Before They Happen',
      content: [
        {
          type: 'text',
          body: 'JavaScript lets you pass ANYTHING to a component. A number where a string was expected? A missing field? You won\'t find out until the app crashes in a user\'s browser. TypeScript adds a safety net: you describe the SHAPE of your data, and the editor tells you about mistakes BEFORE you even run the code.',
        },
        {
          type: 'text',
          body: 'It\'s like building with LEGO. Without TypeScript, any piece fits in any hole — you only find out it\'s wrong when the structure collapses. With TypeScript, each piece has a specific shape — the wrong piece simply won\'t snap in.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// Define the SHAPE of a gem\ntype Gem = {\n  id: number\n  title: string\n  content: string\n  tags: string[]\n}\n\n// Now the component KNOWS what it receives\nfunction GemCard({ gem }: { gem: Gem }) {\n  return (\n    <div>\n      <h3>{gem.title}</h3>\n      <p>{gem.content}</p>\n      {gem.tags.map(tag => <span key={tag}>#{tag}</span>)}\n    </div>\n  )\n}\n\n// TypeScript catches mistakes BEFORE running:\n<GemCard gem={{ id: 1, title: "Rust" }} />\n// Error: Property \'content\' is missing!\n// Error: Property \'tags\' is missing!',
          caption: 'TypeScript catches the missing fields in your EDITOR — before the code even runs.',
        },
        {
          type: 'concept-card',
          term: 'type',
          explanation: 'Describes the shape of data. Like a blueprint that says "a Gem MUST have an id (number), title (string), content (string), and tags (array of strings)." Any data that doesn\'t match gets flagged immediately.',
          example: 'type Gem = {\n  id: number\n  title: string\n  tags: string[]\n}',
        },
        {
          type: 'concept-card',
          term: 'string, number, boolean',
          explanation: 'The basic types. string for text, number for any number (integer or decimal), boolean for true/false. Arrays use the type followed by []: string[] is an array of strings.',
          example: 'let name: string = "Jarvis"\nlet count: number = 42\nlet active: boolean = true\nlet tags: string[] = ["rust", "tauri"]',
        },
        {
          type: 'concept-card',
          term: 'Props typing',
          explanation: 'You describe what props a component accepts. This way TypeScript tells you if you forget a required prop or pass the wrong type.',
          example: 'type Props = {\n  title: string\n  count: number\n  onClose?: () => void  // ? = optional\n}\nfunction Modal({ title, count, onClose }: Props) {}',
        },
        {
          type: 'comparison',
          leftLabel: 'JavaScript',
          rightLabel: 'TypeScript',
          rows: [
            { label: 'Wrong prop type', left: 'Crashes at runtime', right: 'Red squiggly in editor' },
            { label: 'Missing prop', left: 'Shows "undefined"', right: 'Compiler error' },
            { label: 'Autocomplete', left: 'Limited', right: 'Full — knows every field' },
            { label: 'Refactoring', left: 'Scary (might break things)', right: 'Safe (compiler finds all usages)' },
            { label: 'Learning cost', left: 'None', right: 'Small — just add type annotations' },
          ],
        },
        {
          type: 'quiz',
          question: 'What\'s the biggest practical benefit of TypeScript in a React project?',
          options: [
            'The code runs faster',
            'You get autocompletion and catch bugs in your editor before running the code',
            'It makes the bundle smaller',
            'It\'s required for React to work',
          ],
          correctIndex: 1,
          explanation: 'TypeScript gives you instant feedback. Type gem. and your editor shows every available field. Pass the wrong prop? Red underline immediately. Rename a field? The compiler shows every place that needs updating. It\'s like spell-check for your code.',
        },
      ],
    },

    // ── 9. Hooks — Reusable Logic ──
    {
      id: 'hooks',
      title: 'Step 9: Custom Hooks — Reusable Logic',
      content: [
        {
          type: 'text',
          body: 'You\'ll notice patterns repeating: "fetch data, store it in state, show loading while waiting." Instead of copy-pasting this logic into every component, you extract it into a custom hook — a function that starts with "use" and bundles up reusable stateful logic.',
        },
        {
          type: 'text',
          body: 'Think of hooks like kitchen gadgets. useState is a basic knife — you use it everywhere. A custom hook is a specialized tool: "useGems" handles all the gem-fetching logic so your component just says "give me the gems" and doesn\'t worry about how.',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// A custom hook — extracts reusable logic\nfunction useGems() {\n  const [gems, setGems] = useState<Gem[]>([])\n  const [loading, setLoading] = useState(true)\n\n  useEffect(() => {\n    // Fetch gems from the Tauri backend\n    invoke<Gem[]>("get_all_gems").then(data => {\n      setGems(data)\n      setLoading(false)\n    })\n  }, [])  // [] means "run once when component first appears"\n\n  function addGem(title: string, content: string) {\n    invoke<Gem>("create_gem", { title, content }).then(gem => {\n      setGems([...gems, gem])\n    })\n  }\n\n  return { gems, loading, addGem }\n}\n\n// Component is now clean and simple\nfunction App() {\n  const { gems, loading, addGem } = useGems()\n\n  if (loading) return <p>Loading...</p>\n\n  return (\n    <div>\n      <h1>{gems.length} Gems</h1>\n      {gems.map(gem => <GemCard key={gem.id} gem={gem} />)}\n    </div>\n  )\n}',
          caption: 'useGems handles all the data logic. The component just uses the data. Clean separation.',
        },
        {
          type: 'concept-card',
          term: 'useEffect',
          explanation: 'Runs code AFTER the component renders. Used for: fetching data, setting up listeners, timers. The second argument (dependency array) controls WHEN it re-runs. Empty [] = run once on mount.',
          example: 'useEffect(() => {\n  fetchData()   // runs after first render\n}, [])          // [] = only once\n\nuseEffect(() => {\n  search(query) // runs when query changes\n}, [query])     // re-run if query changes',
        },
        {
          type: 'concept-card',
          term: 'Custom hook (use___)',
          explanation: 'A function starting with "use" that bundles stateful logic. It can use useState, useEffect, and other hooks inside it. Components call it to get the data/functions without knowing the internals.',
          example: 'function useRecording() {\n  const [isRecording, setIsRecording] = useState(false)\n  function start() { /* ... */ }\n  function stop() { /* ... */ }\n  return { isRecording, start, stop }\n}',
        },
        {
          type: 'text',
          body: 'Jarvis uses this exact pattern. The useRecording hook manages all recording state and Tauri command calls. The RecordButton component just calls start() and stop() — it doesn\'t know about invoke(), sidecars, or audio processing. Clean separation of "what to show" (component) from "how to do it" (hook).',
        },
        {
          type: 'quiz',
          question: 'Why extract logic into a custom hook instead of putting it all in the component?',
          options: [
            'Hooks are faster than component code',
            'React requires all logic to be in hooks',
            'Reusability: multiple components can use the same hook, and it keeps components focused on UI only',
            'Hooks have better error handling',
          ],
          correctIndex: 2,
          explanation: 'useRecording could be used in RecordButton, RecordingStatus, AND a keyboard shortcut handler — all sharing the same logic. Each component stays simple: it just receives data and renders UI. The hook handles the complexity.',
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'Components (tree structure)',
      file: 'src/App.tsx',
      description: 'The root component that composes Header, RecordButton, GemList, and other children. The whole UI is a tree of nested components.',
    },
    {
      concept: 'State + Events (recording)',
      file: 'src/components/RecordButton.tsx',
      description: 'Uses state to track recording status (idle / recording / processing) and onClick to start/stop. The button label and color change automatically based on state.',
    },
    {
      concept: 'Custom hook (useRecording)',
      file: 'src/hooks/useRecording.ts',
      description: 'Bundles all recording logic: invoke Tauri commands, listen for events, manage state. The RecordButton component just calls start() and stop().',
    },
    {
      concept: 'TypeScript types',
      file: 'src/types.ts',
      description: 'Defines Gem, RecordingStatus, IntelProvider, and other types. Every component and hook is fully typed — the editor catches mistakes immediately.',
    },
    {
      concept: 'Props + Lists (gem display)',
      file: 'src/components/GemList.tsx',
      description: 'Receives an array of gems as props, maps over them to render GemCard components. Each card receives a single gem as props.',
    },
  ],
}
