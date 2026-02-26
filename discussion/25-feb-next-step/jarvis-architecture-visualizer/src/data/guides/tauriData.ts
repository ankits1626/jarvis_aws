import type { Guide } from './types'

export const tauriGuide: Guide = {
  id: 'guide-tauri',
  title: 'Tauri 101',
  subtitle: 'Turn your Rust code into a desktop app with a real UI — the framework that powers Jarvis',
  color: 'teal',
  icon: '⚡',
  sections: [
    // ── 0. What Are We Building? ──
    {
      id: 'big-picture',
      title: 'What Are We Building?',
      content: [
        {
          type: 'text',
          body: 'In Rust 101, you built a gem notebook that runs in a terminal — black screen, typed commands, no buttons. That\'s fine for developers, but real users want to click things. They want a window with a nice UI. Tauri is how you give your Rust program a face.',
        },
        {
          type: 'text',
          body: 'By the end of this guide, you\'ll understand how Tauri connects a pretty web UI to a powerful Rust backend — and exactly how Jarvis uses this to be a desktop app that captures audio, extracts web content, and manages gems, all through a visual interface.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'problem', label: '1. The problem', icon: '🤔' },
            { id: 'arch', label: '2. Two worlds', icon: '🌉' },
            { id: 'setup', label: '3. Set it up', icon: '🔧' },
            { id: 'cmd', label: '4. Talk to Rust', icon: '📡' },
            { id: 'state', label: '5. Shared memory', icon: '🧠' },
            { id: 'events', label: '6. Live updates', icon: '📨' },
            { id: 'plugins', label: '7. Add powers', icon: '🔌' },
            { id: 'config', label: '8. The blueprint', icon: '📋' },
          ],
          connections: [
            { from: 'problem', to: 'arch' },
            { from: 'arch', to: 'setup' },
            { from: 'setup', to: 'cmd' },
            { from: 'cmd', to: 'state' },
            { from: 'state', to: 'events' },
            { from: 'events', to: 'plugins' },
            { from: 'plugins', to: 'config' },
          ],
        },
      ],
    },

    // ── 1. The Problem — Why Can't Rust Show a Window? ──
    {
      id: 'the-problem',
      title: 'Step 1: The Problem — Why Can\'t Rust Just Show a Window?',
      content: [
        {
          type: 'text',
          body: 'Rust is incredible at crunching data, talking to hardware, and being fast. But building a pretty UI with buttons, animations, and layouts? That\'s what the web (HTML/CSS/JavaScript) has spent 30 years perfecting. So instead of reinventing UI from scratch, Tauri says: "Let the web do what it\'s good at (UI), and let Rust do what IT\'s good at (everything else)."',
        },
        {
          type: 'text',
          body: 'Think of a restaurant. The dining room (UI) is where customers sit — it\'s pretty, comfortable, with menus and nice lighting. The kitchen (Rust) is where the real work happens — cooking, prepping, storing food. The waiter (Tauri) carries requests from dining room to kitchen and brings the food back. Nobody expects the chef to also decorate the restaurant.',
        },
        {
          type: 'comparison',
          leftLabel: 'Tauri',
          rightLabel: 'Electron',
          rows: [
            { label: 'What it is', left: 'Rust backend + web UI', right: 'Node.js backend + web UI' },
            { label: 'How big is the app?', left: '~3-10 MB', right: '~150+ MB (bundles a whole browser!)' },
            { label: 'Memory usage', left: '~30-50 MB', right: '~100-300 MB' },
            { label: 'Browser included?', left: 'No — uses the one your OS already has', right: 'Yes — ships an entire Chromium copy' },
            { label: 'Backend speed', left: 'Rust (very fast)', right: 'JavaScript (much slower)' },
            { label: 'Who uses it?', left: 'Jarvis, 1Password, Cody', right: 'VS Code, Slack, Discord' },
          ],
        },
        {
          type: 'text',
          body: 'The big insight: your Mac already has a web browser built in (Safari/WebKit). Tauri says "just use that to show the UI" instead of bundling an entire browser. That\'s why a Tauri app is 3 MB and an Electron app is 150 MB.',
        },
        {
          type: 'quiz',
          question: 'Why is a Tauri app so much smaller than an Electron app?',
          options: [
            'Tauri compresses files better',
            'Tauri skips bundling a browser — it uses the one already on your computer',
            'Tauri doesn\'t support images or CSS',
            'Electron includes extra features nobody needs',
          ],
          correctIndex: 1,
          explanation: 'Electron bundles Chromium (~150MB) inside every app. That\'s like every restaurant building its own road to get there. Tauri uses your OS\'s built-in webview (Safari\'s engine on Mac) — the road already exists.',
        },
      ],
    },

    // ── 2. Two Worlds, One App ──
    {
      id: 'two-worlds',
      title: 'Step 2: Two Worlds, One App',
      content: [
        {
          type: 'text',
          body: 'A Tauri app is literally two separate programs running together. The Frontend is a web page (React, HTML, CSS) that handles everything you see — buttons, text, animations. The Backend is a Rust program that handles everything you DON\'T see — file access, network calls, data processing. They live in different worlds and talk through a bridge.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'user', label: 'You (user)', icon: '👤' },
            { id: 'ui', label: 'Frontend (React)', icon: '🖼️' },
            { id: 'bridge', label: 'IPC Bridge', icon: '🌉' },
            { id: 'rust', label: 'Backend (Rust)', icon: '🦀' },
            { id: 'os', label: 'Your Computer', icon: '💻' },
          ],
          connections: [
            { from: 'user', to: 'ui', label: 'clicks button' },
            { from: 'ui', to: 'bridge', label: '"save this gem"' },
            { from: 'bridge', to: 'rust', label: 'passes the request' },
            { from: 'rust', to: 'os', label: 'writes to disk' },
          ],
        },
        {
          type: 'text',
          body: 'Back to our restaurant analogy: the frontend is the dining room (you see it, you interact with it). The backend is the kitchen (hidden, does the real work). IPC (Inter-Process Communication) is the waiter — carrying orders from your table to the kitchen, and bringing food back.',
        },
        {
          type: 'concept-card',
          term: 'Frontend',
          explanation: 'The part you see and click. Built with web tech (HTML, CSS, JavaScript/React). Runs inside a small browser window (webview). Can\'t access files, network, or hardware directly — it has to ask the backend.',
          example: 'A button that says "Start Recording"\nA list showing your saved gems\nA search bar for finding gems',
        },
        {
          type: 'concept-card',
          term: 'Backend',
          explanation: 'The part you don\'t see. Written in Rust. Has full access to your computer — files, network, processes, hardware. Does all the heavy lifting: saving data, calling APIs, spawning sidecars.',
          example: 'Saving a gem to the SQLite database\nSpawning the audio capture sidecar\nCalling OpenAI for summarization',
        },
        {
          type: 'concept-card',
          term: 'IPC (the bridge)',
          explanation: 'Inter-Process Communication. The messenger between frontend and backend. Frontend says invoke("save_gem", data) and gets back a result. Like calling a waiter — you don\'t walk into the kitchen yourself.',
          example: '// Frontend asks the backend:\nawait invoke("save_gem", { title: "Notes" })\n// Backend processes it, returns a result',
        },
        {
          type: 'text',
          body: 'This separation is also a SECURITY feature. The web UI can\'t just read your files or run programs — it has to go through the Rust backend, which decides what\'s allowed. It\'s like having a bouncer between the public (UI) and the private (your computer).',
        },
      ],
    },

    // ── 3. Setting Up a Tauri Project ──
    {
      id: 'setup',
      title: 'Step 3: Setting It Up',
      content: [
        {
          type: 'text',
          body: 'Before building anything, you need the Tauri CLI. Since Tauri is built on Rust, you install it through Cargo (Rust\'s package manager from Rust 101). The CLI creates projects, runs the dev server, and builds your final app.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Step 1: Install the Tauri CLI\ncargo install tauri-cli --version "^2"\n\n# Verify it\'s installed\ncargo tauri --version\n# tauri-cli 2.x.x',
          caption: 'The --version "^2" ensures you get Tauri 2. It compiles from source, so the first install takes a couple of minutes.',
        },
        {
          type: 'text',
          body: 'Now create a project. The CLI asks a few questions: project name, frontend template (Vanilla, React, Vue, etc.), and language (JavaScript or TypeScript). For learning, pick Vanilla + JavaScript — the simplest option, no build tools needed.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Step 2: Create a new Tauri 2 project\ncargo tauri init\n# It asks: project name, window title, frontend dev URL, frontend dist path\n# Accept defaults for now\n\n# Or use the interactive scaffolder (creates the full folder structure):\nnpm create tauri-app@latest\n# Pick: Vanilla, JavaScript\n\n# Step 3: Run it!\ncd learn_tauri\ncargo tauri dev',
          caption: '`cargo tauri dev` compiles the Rust backend, serves the frontend, and opens the app window.',
        },
        {
          type: 'text',
          body: 'Here\'s what the scaffolder created — the real folder structure of a Tauri 2 project:',
        },
        {
          type: 'code',
          language: 'bash',
          code: 'learn_tauri/\n├── src/                          # FRONTEND (what the user sees)\n│   ├── index.html                # The web page\n│   ├── main.js                   # JavaScript — calls Rust commands\n│   └── styles.css                # Styling\n│\n├── src-tauri/                    # BACKEND (Rust — the real engine)\n│   ├── src/\n│   │   ├── main.rs               # Entry point (just calls lib.rs)\n│   │   └── lib.rs                # App setup — plugins, state, commands\n│   ├── capabilities/\n│   │   └── default.json          # Permissions — what the app is allowed to do\n│   ├── tauri.conf.json           # App config (name, window, sidecars)\n│   └── Cargo.toml                # Rust dependencies',
          caption: 'src/ is vanilla HTML/JS. src-tauri/ is Rust. They talk through the Tauri bridge.',
        },
        {
          type: 'text',
          body: 'Key Tauri 2 difference: there\'s a capabilities/ folder. This is Tauri 2\'s permission system. Your app declares exactly what it\'s allowed to do — like an app asking for camera or location permission on your phone. We\'ll cover this more in the Configuration section.',
        },
        {
          type: 'quiz',
          question: 'In a Tauri project, where do you write code that handles saving a file to disk?',
          options: [
            'src/main.js (the frontend JavaScript)',
            'src-tauri/src/ (the Rust backend)',
            'src/index.html',
            'src-tauri/tauri.conf.json',
          ],
          correctIndex: 1,
          explanation: 'File operations are backend work — the frontend JavaScript can\'t touch the filesystem directly. You\'d write a Rust command in src-tauri/src/ and the frontend would call it through the Tauri bridge.',
        },
      ],
    },

    // ── 4. Commands — The Waiter Takes Your Order ──
    {
      id: 'commands',
      title: 'Step 4: Commands — How the UI Talks to Rust',
      content: [
        {
          type: 'text',
          body: 'This is the most important concept in Tauri. A "command" is a Rust function that the frontend can call. Think of it as a waiter: you (the frontend) write your order on a slip, the waiter (Tauri) carries it to the kitchen (Rust), the kitchen processes it and sends back the result.',
        },
        {
          type: 'text',
          body: 'Let\'s build two commands: "greet" (takes a name, returns a greeting) and "add_numbers" (takes two numbers, returns the sum). You need to touch exactly 3 files:',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'cmd', label: 'commands.rs', icon: '🦀' },
            { id: 'lib', label: 'lib.rs', icon: '🔌' },
            { id: 'main', label: 'main.js', icon: '📄' },
          ],
          connections: [
            { from: 'cmd', to: 'lib', label: '1. write → 2. register' },
            { from: 'lib', to: 'main', label: '3. call from frontend' },
          ],
        },
        {
          type: 'text',
          body: 'FILE 1 — src-tauri/src/commands.rs (create this file)\nThis is the kitchen. Write the actual functions that do the work. The #[tauri::command] tag tells Tauri "the frontend is allowed to call this."',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// src-tauri/src/commands.rs\n\n#[tauri::command]\npub fn greet(name: String) -> String {\n    format!("Hello, {}! Welcome to Jarvis.", name)\n}\n\n#[tauri::command]\npub fn add_numbers(a: i32, b: i32) -> i32 {\n    a + b\n}',
          caption: 'src-tauri/src/commands.rs — each #[tauri::command] function becomes callable from JavaScript',
        },
        {
          type: 'text',
          body: 'FILE 2 — src-tauri/src/lib.rs\nThis is the restaurant\'s menu. You REGISTER commands here. If a command isn\'t listed, the frontend can\'t call it — even if the function exists in commands.rs.',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// src-tauri/src/lib.rs\n\nmod commands;  // "I have a commands.rs file"\n\nuse commands::{greet, add_numbers};\n\npub fn run() {\n    tauri::Builder::default()\n        .invoke_handler(tauri::generate_handler![\n            greet,        // frontend can call this\n            add_numbers,  // and this\n        ])\n        .run(tauri::generate_context!())\n        .expect("error while running tauri application");\n}',
          caption: 'src-tauri/src/lib.rs — generate_handler! is the menu of available commands',
        },
        {
          type: 'text',
          body: 'FILE 3 — src/main.js\nThis is the dining room. The customer places orders using invoke(). In Tauri 2, when you set "withGlobalTauri": true in tauri.conf.json, the invoke function is available on window.__TAURI__ — no npm packages needed.',
        },
        {
          type: 'code',
          language: 'javascript',
          code: '// src/main.js\n\n// Tauri 2 exposes its API on window.__TAURI__\n// (enabled by "withGlobalTauri": true in tauri.conf.json)\nconst { invoke } = window.__TAURI__.core;\n\nasync function greet() {\n  // Call the Rust function!\n  const greeting = await invoke("greet", { name: "Ankit" });\n  document.querySelector("#greet-msg").textContent = greeting;\n  // Shows: "Hello, Ankit! Welcome to Jarvis."\n}\n\nasync function add() {\n  const sum = await invoke("add_numbers", { a: 10, b: 32 });\n  document.querySelector("#add-result").textContent = `10 + 32 = ${sum}`;\n  // Shows: "10 + 32 = 42"\n}',
          caption: 'src/main.js — invoke() sends a request to Rust and returns the result. No npm install needed!',
        },
        {
          type: 'text',
          body: 'That\'s the complete loop. Three files, three roles:\n\n• commands.rs → Write the function (the kitchen recipe)\n• lib.rs → Register the function (put it on the menu)\n• main.js → Call the function (place the order)\n\nMiss any step and it won\'t work. Jarvis has ~37 commands — all following this exact same pattern.',
        },
        {
          type: 'text',
          body: 'Note: if you use a framework like React with npm, you can also install @tauri-apps/api and use import { invoke } from "@tauri-apps/api/core" instead of window.__TAURI__. Both work — Jarvis uses the npm approach since it uses React.',
        },
        {
          type: 'interactive-code',
          language: 'javascript',
          starterCode: '// You\'ve written a Rust command called "search_gems"\n// that takes { query: String } and returns Vec<String>\n//\n// Write the JavaScript invoke() call.\n// Search for "rust notes" and log the results.\nconst { invoke } = window.__TAURI__.core;\n\nasync function search() {\n  // Your code here\n}',
          solution: 'const { invoke } = window.__TAURI__.core;\n\nasync function search() {\n  const results = await invoke("search_gems", {\n    query: "rust notes",\n  });\n  console.log(results);\n}',
          hint: 'Use await invoke("command_name", { param: value }). The Rust Vec<String> comes back as a JavaScript array of strings.',
          validator: (input: string) => {
            return input.includes('invoke') && input.includes('search_gems') && input.includes('query')
          },
        },
        {
          type: 'quiz',
          question: 'You wrote a #[tauri::command] function called "delete_gem" in commands.rs but forgot to add it to generate_handler! in lib.rs. What happens when JavaScript calls invoke("delete_gem")?',
          options: [
            'It works anyway — Tauri auto-discovers commands',
            'It silently does nothing',
            'The invoke() returns a rejected Promise — command not found',
            'The app crashes',
          ],
          correctIndex: 2,
          explanation: 'Tauri only executes commands explicitly listed in generate_handler!. The function exists in Rust but the frontend can\'t reach it. This is a security feature — you control exactly what the UI can do. Always: write it, register it, then call it.',
        },
      ],
    },

    // ── 5. State — The Kitchen's Shared Fridge ──
    {
      id: 'state',
      title: 'Step 5: State — The Shared Fridge',
      content: [
        {
          type: 'text',
          body: 'Commands do work — but they need ACCESS to things. The `create_gem` command needs the database connection. The `start_recording` command needs the audio sidecar handle. Where do all these shared resources live? In Tauri\'s "state" — a shared fridge that every command can reach into.',
        },
        {
          type: 'text',
          body: 'In our restaurant analogy: every chef needs access to the same fridge, the same pantry, the same oven. You don\'t give each chef their own fridge. You have ONE shared kitchen with shared equipment. Tauri\'s state is that shared kitchen.',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// Define what\'s in the shared kitchen (state)\nstruct AppState {\n    gem_store: Mutex<GemStore>,          // the database (one shared copy)\n    recording: Mutex<RecordingStatus>,   // is audio recording active?\n    ai_provider: RwLock<Option<Box<dyn IntelProvider>>>,  // the AI engine\n}\n\n// Put it in the kitchen (register with Tauri)\ntauri::Builder::default()\n    .manage(AppState { /* ... */ })   // <-- "here\'s the shared fridge"\n    .invoke_handler(/* commands */)',
          caption: '.manage() registers the shared state. Every command can now access it.',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// A command reaches into the shared fridge\n#[tauri::command]\nasync fn create_gem(\n    state: State<\'_, AppState>,  // <-- "give me access to the fridge"\n    title: String,\n    content: String,\n) -> Result<String, String> {\n    // Lock the gem store (only one command can use it at a time)\n    let store = state.gem_store.lock().unwrap();\n    store.save(title, content).map_err(|e| e.to_string())\n}',
          caption: 'State<\'_, AppState> is Tauri\'s way of saying "this command needs the shared state"',
        },
        {
          type: 'text',
          body: 'Notice the `Mutex` wrapper? Remember from Rust 101: multiple commands might run at the same time (on different threads). Mutex is like a "take a number" system at the deli counter — only one command can access the gem store at a time. This prevents two commands from writing to the database simultaneously and corrupting data.',
        },
        {
          type: 'concept-card',
          term: '.manage(state)',
          explanation: 'Registers shared data with Tauri. Like stocking the kitchen fridge before the restaurant opens. You do this once at startup.',
          example: 'tauri::Builder::default()\n    .manage(MyState { db: open_database() })',
        },
        {
          type: 'concept-card',
          term: 'State<\'_, T>',
          explanation: 'A command parameter that says "I need access to the shared state." Tauri automatically provides it — you just declare it as a parameter.',
          example: '#[tauri::command]\nfn my_cmd(state: State<\'_, AppState>) {\n    let data = state.db.lock().unwrap();\n}',
        },
        {
          type: 'quiz',
          question: 'Why is the gem store wrapped in Mutex inside AppState?',
          options: [
            'To encrypt the database',
            'Because two commands might try to write to the database at the same time — Mutex ensures only one can access it at a time',
            'Because Rust requires all structs to use Mutex',
            'To make the state persistent across app restarts',
          ],
          correctIndex: 1,
          explanation: 'Imagine two users click "Save Gem" at the exact same moment. Without Mutex, both commands would try to write to SQLite simultaneously — corrupting data. Mutex makes them take turns, like a bathroom lock.',
        },
      ],
    },

    // ── 6. Events — The Notification Bell ──
    {
      id: 'events',
      title: 'Step 6: Events — Live Updates',
      content: [
        {
          type: 'text',
          body: 'Commands work like a question-and-answer: the frontend asks, the backend responds. But what about things that happen WITHOUT the frontend asking? Like: "the recording just finished" or "transcription is 50% done." The backend needs a way to TAP the frontend on the shoulder. That\'s events.',
        },
        {
          type: 'text',
          body: 'Think about ordering a pizza for delivery. You don\'t call the restaurant every 30 seconds asking "is it ready yet?" Instead, the delivery app SENDS YOU a notification: "Your pizza is being prepared", "Out for delivery", "Arrived!" Events work the same way.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'rust', label: 'Rust Backend', icon: '🦀' },
            { id: 'event', label: 'emit("recording-done")', icon: '📨' },
            { id: 'ui', label: 'React UI', icon: '🖼️' },
            { id: 'update', label: 'Shows "Done!"', icon: '✅' },
          ],
          connections: [
            { from: 'rust', to: 'event', label: 'something happened!' },
            { from: 'event', to: 'ui', label: 'listener fires' },
            { from: 'ui', to: 'update', label: 'updates screen' },
          ],
        },
        {
          type: 'code',
          language: 'rust',
          code: '// BACKEND: Send a notification to the frontend\n// "Hey UI, the recording just finished!"\napp_handle.emit("recording-done", RecordingResult {\n    duration_secs: 45,\n    file_path: "/tmp/recording.wav".to_string(),\n}).unwrap();',
          caption: 'emit() sends an event from Rust to the frontend — the frontend doesn\'t need to ask for it',
        },
        {
          type: 'code',
          language: 'typescript',
          code: '// FRONTEND: Listen for the notification\nimport { listen } from "@tauri-apps/api/event";\n\n// "Whenever the backend says recording is done, run this code"\nconst unlisten = await listen("recording-done", (event) => {\n  const result = event.payload;\n  showNotification(`Recording done! ${result.duration_secs}s`);\n});\n\n// Later, when you don\'t need it anymore:\nunlisten();  // stop listening',
          caption: 'listen() subscribes to events. It returns a cleanup function to stop listening.',
        },
        {
          type: 'comparison',
          leftLabel: 'Commands (invoke)',
          rightLabel: 'Events (emit/listen)',
          rows: [
            { label: 'Direction', left: 'Frontend → Backend → Frontend', right: 'Backend → Frontend' },
            { label: 'Who starts it?', left: 'Frontend asks', right: 'Backend notifies' },
            { label: 'Pattern', left: 'Request / Response', right: 'Fire and forget' },
            { label: 'Analogy', left: 'Asking the waiter a question', right: 'The kitchen rings the bell' },
            { label: 'Jarvis example', left: 'invoke("create_gem")', right: 'emit("transcription-progress")' },
          ],
        },
        {
          type: 'text',
          body: 'Jarvis uses events heavily for recording: "recording started", "audio level update", "transcription 30% done", "transcription complete." Without events, the UI would have no idea what\'s happening in the background — it would just freeze with a spinner.',
        },
        {
          type: 'quiz',
          question: 'When would you use an event instead of a command?',
          options: [
            'When the frontend needs data from the backend',
            'When the backend needs to push updates to the frontend without being asked',
            'When you want to save a file',
            'When the app starts',
          ],
          correctIndex: 1,
          explanation: 'Commands are "frontend asks, backend answers." Events are "backend tells the frontend something happened." Use events for progress updates, state changes, and notifications — things the backend initiates.',
        },
      ],
    },

    // ── 7. Plugins — Adding Superpowers ──
    {
      id: 'plugins',
      title: 'Step 7: Plugins — Adding Superpowers',
      content: [
        {
          type: 'text',
          body: 'Tauri keeps your app locked down by default — the frontend can\'t access the filesystem, can\'t spawn processes, can\'t show native dialogs. You have to explicitly add these capabilities through plugins. Think of it like installing apps on a new phone — it starts with nothing, and you add what you need.',
        },
        {
          type: 'text',
          body: 'Why this restriction? Security. A web page shouldn\'t be able to read your files or run programs without permission. Tauri makes you opt-in to each capability, so your app only has the powers it actually needs.',
        },
        {
          type: 'concept-card',
          term: 'tauri-plugin-shell',
          explanation: 'The ability to run external programs. Jarvis NEEDS this to launch its Swift sidecars (JarvisListen for audio, IntelligenceKit for AI). Without this plugin, no sidecar would start.',
          example: '// In lib.rs:\n.plugin(tauri_plugin_shell::init())\n\n// Now you can spawn processes:\napp.shell().sidecar("jarvis-listen").spawn()',
        },
        {
          type: 'concept-card',
          term: 'tauri-plugin-dialog',
          explanation: 'Native file-picker dialogs (Open, Save). The OS-native ones, not web-style file inputs. Used when exporting gems or choosing an audio file.',
          example: '.plugin(tauri_plugin_dialog::init())\n\n// Shows a real macOS file picker',
        },
        {
          type: 'concept-card',
          term: 'tauri-plugin-notification',
          explanation: 'Send native OS notifications — the ones that appear in your notification center. Jarvis uses this to tell you "Recording complete!" even when the app is in the background.',
          example: '.plugin(tauri_plugin_notification::init())\n\n// Shows a macOS notification bubble',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// Plugins are installed in the builder chain — like adding\n// appliances to your kitchen before opening the restaurant\ntauri::Builder::default()\n    .plugin(tauri_plugin_shell::init())         // can run programs\n    .plugin(tauri_plugin_dialog::init())        // can show file pickers\n    .plugin(tauri_plugin_notification::init())  // can send notifications\n    .manage(app_state)                          // shared state\n    .invoke_handler(tauri::generate_handler![   // commands\n        create_gem, start_recording, /* ... */\n    ])',
          caption: 'Each .plugin() line adds one superpower. Without it, that capability doesn\'t exist.',
        },
        {
          type: 'quiz',
          question: 'What happens if Jarvis tries to spawn the JarvisListen sidecar but tauri-plugin-shell is NOT installed?',
          options: [
            'It works anyway — Rust can always spawn processes',
            'The app crashes on startup',
            'The spawn call fails with a permission error — the capability wasn\'t enabled',
            'The sidecar runs but can\'t communicate back',
          ],
          correctIndex: 2,
          explanation: 'Without the shell plugin, Tauri doesn\'t have the capability to spawn external processes. The call would fail. This is intentional — apps should only have the powers they explicitly declare. It\'s the principle of least privilege.',
        },
      ],
    },

    // ── 8. Configuration — The Blueprint ──
    {
      id: 'configuration',
      title: 'Step 8: Configuration — The Blueprint',
      content: [
        {
          type: 'text',
          body: 'Every Tauri app has a `tauri.conf.json` file — the master blueprint. It tells Tauri: what\'s your app called? How big should the window be? What security rules apply? What sidecars should be bundled? Think of it as the building permit for your app.',
        },
        {
          type: 'code',
          language: 'json',
          code: '{\n  "productName": "Jarvis",\n  "identifier": "com.jarvis.app",\n  "build": {\n    "devUrl": "http://localhost:1420",\n    "frontendDist": "../dist"\n  },\n  "app": {\n    "windows": [\n      {\n        "title": "Jarvis",\n        "width": 1200,\n        "height": 800\n      }\n    ],\n    "security": {\n      "csp": "default-src \'self\'; connect-src \'self\' https://api.openai.com"\n    }\n  },\n  "bundle": {\n    "externalBin": [\n      "binaries/jarvis-listen",\n      "binaries/intelligence-kit"\n    ]\n  }\n}',
          caption: 'Jarvis\'s tauri.conf.json (simplified). Each section controls a different aspect of the app.',
        },
        {
          type: 'concept-card',
          term: 'externalBin (sidecars)',
          explanation: 'Lists executables that get bundled WITH your app. When you build the app, Tauri copies these binaries into the package so they\'re available at runtime. Jarvis bundles two Swift programs here.',
          example: '"externalBin": [\n  "binaries/jarvis-listen",\n  "binaries/intelligence-kit"\n]',
        },
        {
          type: 'concept-card',
          term: 'CSP (Content Security Policy)',
          explanation: 'Security rules for the web UI. Controls what websites the frontend can talk to. Jarvis allows connections to OpenAI\'s API but blocks everything else — preventing malicious scripts from sending your data somewhere bad.',
          example: '"csp": "default-src \'self\'; connect-src \'self\' https://api.openai.com"',
        },
        {
          type: 'concept-card',
          term: 'devUrl vs frontendDist',
          explanation: 'During development, the frontend runs on a local dev server (localhost:1420). In production, Tauri loads the built files from the dist/ folder. This setting tells Tauri where to look.',
          example: '"devUrl": "http://localhost:1420"  // dev\n"frontendDist": "../dist"          // production',
        },
        {
          type: 'text',
          body: 'Everything comes together in this one file. When you run `tauri build`, it reads this blueprint and creates a distributable app: the Rust backend compiled to native code, the frontend bundled as HTML/CSS/JS, sidecars copied in, all wrapped in a macOS .app (or Windows .exe).',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'conf', label: 'tauri.conf.json', icon: '📋' },
            { id: 'build', label: 'tauri build', icon: '🔨' },
            { id: 'app', label: 'Jarvis.app', icon: '📦' },
          ],
          connections: [
            { from: 'conf', to: 'build', label: 'reads blueprint' },
            { from: 'build', to: 'app', label: 'creates package' },
          ],
        },
      ],
    },

    // ── 9. Putting It All Together ──
    {
      id: 'putting-together',
      title: 'The Full Picture',
      content: [
        {
          type: 'text',
          body: 'Let\'s trace one complete action through the entire Tauri stack: you click "Start Recording" in Jarvis.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'click', label: '1. Click button', icon: '👆' },
            { id: 'invoke', label: '2. invoke()', icon: '📡' },
            { id: 'cmd', label: '3. Rust command', icon: '🦀' },
            { id: 'state', label: '4. Check state', icon: '🧠' },
            { id: 'plugin', label: '5. Shell plugin', icon: '🔌' },
            { id: 'sidecar', label: '6. Spawn sidecar', icon: '🎙️' },
            { id: 'event', label: '7. Emit events', icon: '📨' },
            { id: 'ui', label: '8. UI updates', icon: '🖼️' },
          ],
          connections: [
            { from: 'click', to: 'invoke', label: 'frontend' },
            { from: 'invoke', to: 'cmd', label: 'IPC bridge' },
            { from: 'cmd', to: 'state', label: 'lock mutex' },
            { from: 'state', to: 'plugin', label: 'need to spawn' },
            { from: 'plugin', to: 'sidecar', label: 'JarvisListen' },
            { from: 'sidecar', to: 'event', label: 'audio ready' },
            { from: 'event', to: 'ui', label: 'show recording' },
          ],
        },
        {
          type: 'text',
          body: '(1) You click the record button. (2) React calls invoke("start_recording"). (3) Tauri routes it to the Rust start_recording command. (4) The command checks AppState — is anything already recording? (5) It uses the shell plugin to spawn JarvisListen. (6) The Swift sidecar starts capturing audio. (7) Rust emits "recording-started" event. (8) React hears the event and shows the recording indicator. All of this happens in milliseconds.',
        },
        {
          type: 'quiz',
          question: 'Which Tauri concepts does the "Start Recording" flow use?',
          options: [
            'Only commands',
            'Commands + state + plugins + events — all of them',
            'Only events',
            'Only the configuration file',
          ],
          correctIndex: 1,
          explanation: 'One button click touches every concept: invoke (command), State (check recording status), plugin (shell for sidecar), and events (notify the UI). That\'s the power of Tauri — each piece has a clear job and they compose together.',
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'The Builder Chain',
      file: 'src-tauri/src/lib.rs',
      description: 'The entire Tauri app is configured here: plugins → state → commands → run. This is the "kitchen setup" that happens before the restaurant opens.',
    },
    {
      concept: 'Commands (~37 total)',
      file: 'src-tauri/src/commands.rs',
      description: 'Every action the frontend can trigger: create/search/delete gems, start/stop recording, extract from browser, configure AI provider, and more.',
    },
    {
      concept: 'Events (recording status)',
      file: 'src-tauri/src/recording.rs',
      description: 'Recording emits events as the state changes: started → capturing audio → transcription progress → done. The React UI subscribes to these.',
    },
    {
      concept: 'Configuration + Sidecars',
      file: 'src-tauri/tauri.conf.json',
      description: 'Declares the app name, window size, security CSP, and bundles the JarvisListen + IntelligenceKit Swift sidecars.',
    },
    {
      concept: 'Frontend invocations',
      file: 'src/App.tsx',
      description: 'The React frontend uses invoke() to call Rust commands and listen() to subscribe to backend events. Every button click goes through this bridge.',
    },
  ],
}
