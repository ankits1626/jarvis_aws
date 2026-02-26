import type { Guide } from './types'

export const swiftGuide: Guide = {
  id: 'guide-swift',
  title: 'Swift 101',
  subtitle: 'Apple\'s language for macOS APIs — powering Jarvis\'s audio capture and on-device AI',
  color: 'orange',
  icon: '🐦',
  sections: [
    // ── 1. Why Swift for Sidecars? ──
    {
      id: 'why-swift',
      title: 'Why Swift for Sidecars?',
      content: [
        {
          type: 'text',
          body: 'Jarvis is a Rust app, so why use Swift at all? Because Apple\'s most powerful APIs — ScreenCaptureKit for audio capture, Foundation Models for on-device AI — are Swift-only. Rather than fighting FFI boundaries, Jarvis runs Swift code as separate sidecar processes that communicate with Rust over stdin/stdout.',
        },
        {
          type: 'comparison',
          leftLabel: 'Swift (Sidecar)',
          rightLabel: 'Rust (Direct)',
          rows: [
            { label: 'Apple API access', left: 'Full (first-class)', right: 'Limited (C bridges only)' },
            { label: 'ScreenCaptureKit', left: 'Native support', right: 'Not available' },
            { label: 'Foundation Models', left: 'Native support', right: 'Not available' },
            { label: 'Performance', left: 'Excellent for Apple APIs', right: 'Excellent for computation' },
            { label: 'Integration cost', left: 'Sidecar IPC overhead', right: 'None (same process)' },
            { label: 'Error isolation', left: 'Process boundary (safe)', right: 'Shared memory (risky)' },
          ],
        },
        {
          type: 'quiz',
          question: 'Why doesn\'t Jarvis call ScreenCaptureKit directly from Rust?',
          options: [
            'Rust is too slow for audio processing',
            'ScreenCaptureKit is a Swift-only API with no C bridge',
            'Tauri doesn\'t support audio',
            'Swift is faster than Rust for all tasks',
          ],
          correctIndex: 1,
          explanation: 'Apple\'s ScreenCaptureKit is only available through Swift/Objective-C. While Rust can call C APIs, there\'s no stable C bridge for ScreenCaptureKit. A Swift sidecar is the cleanest approach.',
        },
      ],
    },

    // ── 2. Swift Basics ──
    {
      id: 'swift-basics',
      title: 'Swift Basics',
      content: [
        {
          type: 'text',
          body: 'Swift is a modern, type-safe language designed by Apple. If you know TypeScript or Rust, many concepts will feel familiar. Here are the key building blocks used in Jarvis\'s Swift sidecars.',
        },
        {
          type: 'concept-card',
          term: 'Optionals (T?)',
          explanation: 'Like Rust\'s Option<T>. A value that might be nil. You must unwrap safely before use.',
          example: 'var name: String? = nil\nif let n = name {\n    print(n) // only runs if not nil\n}',
        },
        {
          type: 'concept-card',
          term: 'Structs',
          explanation: 'Value types that hold data. Like Rust structs but with default memberwise initializers.',
          example: 'struct AudioConfig {\n    let sampleRate: Int\n    let channels: Int\n    var format: String = "wav"\n}',
        },
        {
          type: 'concept-card',
          term: 'Enums with Associated Values',
          explanation: 'Like Rust\'s enums — each case can carry different data. Essential for modeling states and messages.',
          example: 'enum SidecarMessage {\n    case audioReady(path: String)\n    case error(message: String)\n    case status(percent: Double)\n}',
        },
        {
          type: 'concept-card',
          term: 'Protocols',
          explanation: 'Like Rust\'s traits or TypeScript interfaces. Define a contract that types must fulfill.',
          example: 'protocol AudioCapture {\n    func start() async throws\n    func stop() async\n    var isRunning: Bool { get }\n}',
        },
        {
          type: 'code',
          language: 'swift',
          code: '// Jarvis: Message types for sidecar communication\nstruct SidecarRequest: Codable {\n    let action: String\n    let params: [String: String]?\n}\n\nstruct SidecarResponse: Codable {\n    let type: String\n    let data: [String: AnyCodable]?\n    let error: String?\n}',
          caption: 'Codable structs for JSON serialization over stdin/stdout',
        },
      ],
    },

    // ── 3. Async/Await in Swift ──
    {
      id: 'async-await',
      title: 'Async/Await in Swift',
      content: [
        {
          type: 'text',
          body: 'Swift\'s structured concurrency model is similar to Rust\'s async/await but with some key differences. Both avoid callback hell, but Swift uses actors for thread safety while Rust uses ownership.',
        },
        {
          type: 'comparison',
          leftLabel: 'Swift async',
          rightLabel: 'Rust async',
          rows: [
            { label: 'Keyword', left: 'async / await', right: 'async / .await' },
            { label: 'Error handling', left: 'throws / try', right: 'Result<T, E> / ?' },
            { label: 'Concurrency', left: 'Task { } / actors', right: 'tokio::spawn / Arc<Mutex>' },
            { label: 'Cancellation', left: 'Built-in (Task.cancel)', right: 'Manual (CancellationToken)' },
            { label: 'Runtime', left: 'Built into Swift', right: 'External (Tokio)' },
          ],
        },
        {
          type: 'code',
          language: 'swift',
          code: '// Jarvis: Async audio capture in JarvisListen\nfunc startCapture() async throws {\n    let stream = try await SCShareableContent.current\n    let config = SCStreamConfiguration()\n    config.sampleRate = 16000\n    config.channelCount = 1\n    \n    let capture = SCStream(filter: filter, configuration: config)\n    try await capture.startCapture()\n    \n    // Audio buffers arrive via delegate callbacks\n}',
          caption: 'Swift async/await for ScreenCaptureKit audio capture',
        },
        {
          type: 'concept-card',
          term: 'Task { }',
          explanation: 'Launches a new concurrent unit of work. Similar to tokio::spawn in Rust.',
          example: 'Task {\n    let result = await processAudio(buffer)\n    sendResponse(result)\n}',
        },
        {
          type: 'concept-card',
          term: 'Actor',
          explanation: 'A reference type that protects its mutable state from concurrent access. Like Rust\'s Arc<Mutex<T>> but built into the language.',
          example: 'actor AudioManager {\n    var isRecording = false\n    func start() { isRecording = true }\n    // Only one task can run a method at a time\n}',
        },
      ],
    },

    // ── 4. ScreenCaptureKit ──
    {
      id: 'screencapturekit',
      title: 'ScreenCaptureKit',
      content: [
        {
          type: 'text',
          body: 'ScreenCaptureKit (SCK) is Apple\'s framework for capturing screen and audio content. Jarvis uses it to capture system audio — what\'s playing through your speakers — so it can transcribe meetings, podcasts, and videos.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'permission', label: 'Permission Request', icon: '🔐' },
            { id: 'content', label: 'SCShareableContent', icon: '📋' },
            { id: 'filter', label: 'SCContentFilter', icon: '🔍' },
            { id: 'stream', label: 'SCStream', icon: '🎵' },
            { id: 'buffer', label: 'Audio Buffers', icon: '📊' },
          ],
          connections: [
            { from: 'permission', to: 'content', label: 'granted' },
            { from: 'content', to: 'filter', label: 'select source' },
            { from: 'filter', to: 'stream', label: 'configure' },
            { from: 'stream', to: 'buffer', label: 'delegate callback' },
          ],
        },
        {
          type: 'code',
          language: 'swift',
          code: '// The audio capture pipeline\nclass AudioCaptureDelegate: NSObject, SCStreamDelegate, SCStreamOutput {\n    func stream(_ stream: SCStream, \n                didOutputSampleBuffer buffer: CMSampleBuffer,\n                of type: SCStreamOutputType) {\n        guard type == .audio else { return }\n        \n        // Convert CMSampleBuffer to PCM data\n        let audioBuffer = buffer.asPCMBuffer()\n        \n        // Write to file (NOT stdout — learned the hard way!)\n        audioFile.write(audioBuffer)\n    }\n}',
          caption: 'Audio arrives as CMSampleBuffer — written to file to avoid stdout corruption',
        },
        {
          type: 'quiz',
          question: 'Why does JarvisListen write audio data to a file instead of stdout?',
          options: [
            'Files are faster than stdout',
            'Binary PCM data contains 0x0A bytes that look like newlines, corrupting the NDJSON protocol on stdout',
            'macOS doesn\'t allow audio on stdout',
            'The audio data is too large for stdout',
          ],
          correctIndex: 1,
          explanation: 'PCM audio data contains raw bytes including 0x0A (the newline character). Since stdout also uses newlines to separate NDJSON messages, binary audio data would corrupt the protocol. Using a temp file avoids this.',
        },
      ],
    },

    // ── 5. Foundation Models ──
    {
      id: 'foundation-models',
      title: 'Foundation Models',
      content: [
        {
          type: 'text',
          body: 'Apple\'s Foundation Models framework (macOS 26+) provides on-device LLM capabilities. Jarvis uses it through the IntelligenceKit sidecar for summarization and tagging — completely offline, no API keys needed.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'rust', label: 'Jarvis (Rust)', icon: '🦀' },
            { id: 'ik', label: 'IntelligenceKit', icon: '🧠' },
            { id: 'fm', label: 'Foundation Models', icon: '🍎' },
            { id: 'result', label: 'Summary + Tags', icon: '📝' },
          ],
          connections: [
            { from: 'rust', to: 'ik', label: 'NDJSON request' },
            { from: 'ik', to: 'fm', label: 'on-device inference' },
            { from: 'fm', to: 'result', label: 'generates' },
          ],
        },
        {
          type: 'concept-card',
          term: '@Generable',
          explanation: 'A Swift macro that makes a struct conforming to a schema that the LLM can generate. The model outputs structured data matching your Swift types.',
          example: '@Generable\nstruct Summary {\n    let title: String\n    let bulletPoints: [String]\n    let tags: [String]\n}',
        },
        {
          type: 'concept-card',
          term: '@Guide',
          explanation: 'A property wrapper that provides instructions to the model about how to fill a field. Like a prompt for each struct property.',
          example: '@Generable\nstruct Summary {\n    @Guide(description: "A concise 1-line title")\n    let title: String\n}',
        },
        {
          type: 'code',
          language: 'swift',
          code: '// IntelligenceKit: On-device summarization\nlet session = LanguageModelSession()\nlet response = try await session.respond(\n    to: "Summarize this transcript: \\(text)",\n    generating: Summary.self\n)\n// response.title, response.bulletPoints, response.tags\n// are all typed — no JSON parsing needed!',
          caption: 'Foundation Models generates typed Swift structs directly',
        },
      ],
    },

    // ── 6. Stdin/Stdout Communication ──
    {
      id: 'stdin-stdout',
      title: 'Stdin/Stdout Communication',
      content: [
        {
          type: 'text',
          body: 'Jarvis\'s Swift sidecars communicate with the Rust backend via NDJSON (Newline-Delimited JSON) over stdin/stdout. Each line is a complete JSON message. The Rust side writes to stdin, reads from stdout. Simple, reliable, cross-platform.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'rust', label: 'Rust (parent)', icon: '🦀' },
            { id: 'stdin', label: 'stdin →', icon: '📥' },
            { id: 'swift', label: 'Swift (sidecar)', icon: '🐦' },
            { id: 'stdout', label: '← stdout', icon: '📤' },
          ],
          connections: [
            { from: 'rust', to: 'stdin', label: 'write JSON' },
            { from: 'stdin', to: 'swift', label: 'read line' },
            { from: 'swift', to: 'stdout', label: 'write JSON' },
          ],
        },
        {
          type: 'code',
          language: 'swift',
          code: '// Swift sidecar: Reading from stdin, writing to stdout\nwhile let line = readLine() {\n    guard let data = line.data(using: .utf8),\n          let request = try? JSONDecoder().decode(\n              SidecarRequest.self, from: data\n          ) else { continue }\n    \n    let response = await handleRequest(request)\n    let json = try JSONEncoder().encode(response)\n    print(String(data: json, encoding: .utf8)!)\n    fflush(stdout) // Critical: flush immediately!\n}',
          caption: 'The NDJSON read-process-respond loop in a Swift sidecar',
        },
        {
          type: 'interactive-code',
          language: 'json',
          starterCode: '// Write a NDJSON request to start audio capture\n// Action: "start_capture"\n// Params: sample_rate = "16000", format = "wav"\n',
          solution: '{"action":"start_capture","params":{"sample_rate":"16000","format":"wav"}}',
          hint: 'NDJSON is just regular JSON on a single line. Use the SidecarRequest format: { "action": "...", "params": { ... } }',
          validator: (input: string) => {
            try {
              const parsed = JSON.parse(input.trim())
              return parsed.action === 'start_capture' && parsed.params?.sample_rate === '16000'
            } catch { return false }
          },
        },
      ],
    },

    // ── 7. Process Lifecycle ──
    {
      id: 'process-lifecycle',
      title: 'Process Lifecycle',
      content: [
        {
          type: 'text',
          body: 'A sidecar goes through a predictable lifecycle: spawn → initialize → communicate → terminate. Jarvis must handle each phase correctly, including crash recovery and graceful shutdown.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'spawn', label: 'Spawn', icon: '🚀' },
            { id: 'init', label: 'Initialize', icon: '⚙️' },
            { id: 'ready', label: 'Ready (listening)', icon: '✅' },
            { id: 'active', label: 'Processing', icon: '🔄' },
            { id: 'terminate', label: 'Terminate', icon: '🛑' },
          ],
          connections: [
            { from: 'spawn', to: 'init', label: 'process starts' },
            { from: 'init', to: 'ready', label: 'sends "ready"' },
            { from: 'ready', to: 'active', label: 'receives request' },
            { from: 'active', to: 'terminate', label: 'kill / exit' },
          ],
        },
        {
          type: 'code',
          language: 'swift',
          code: '// Swift sidecar: Lifecycle management\n@main\nstruct JarvisListen {\n    static func main() async {\n        // 1. Initialize\n        let capture = AudioCapture()\n        \n        // 2. Signal ready\n        print(#"{"type":"ready"}"#)\n        fflush(stdout)\n        \n        // 3. Process commands\n        while let line = readLine() {\n            await handleCommand(line, capture: capture)\n        }\n        \n        // 4. Clean up (stdin closed = parent exited)\n        await capture.stop()\n    }\n}',
          caption: 'The sidecar lifecycle — ready signal, command loop, cleanup',
        },
        {
          type: 'quiz',
          question: 'What happens when a Swift sidecar\'s stdin closes?',
          options: [
            'Nothing — it keeps running forever',
            'It crashes with an error',
            'readLine() returns nil, exiting the while loop — the sidecar cleans up and exits',
            'macOS force-kills the process',
          ],
          correctIndex: 2,
          explanation: 'When the parent (Rust) process closes the sidecar\'s stdin, readLine() returns nil. The while-let loop exits, and the sidecar runs its cleanup code (stopping audio capture, closing files) before exiting.',
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'Audio Capture (ScreenCaptureKit)',
      file: 'jarvis-listen/Sources/',
      description: 'The JarvisListen Swift sidecar captures system audio using ScreenCaptureKit and writes PCM data to temp files.',
    },
    {
      concept: 'On-Device AI (Foundation Models)',
      file: 'intelligence-kit/Sources/',
      description: 'The IntelligenceKit sidecar uses Apple\'s Foundation Models for on-device summarization and tagging.',
    },
    {
      concept: 'NDJSON Protocol',
      file: 'src-tauri/src/recording.rs',
      description: 'The Rust side reads NDJSON from JarvisListen\'s stdout and writes commands to its stdin.',
    },
    {
      concept: 'Sidecar Spawning',
      file: 'src-tauri/src/lib.rs',
      description: 'Swift sidecars are spawned using Tauri\'s shell plugin with the sidecar() API.',
    },
    {
      concept: 'Stdout Corruption Fix',
      file: 'jarvis-listen/Sources/',
      description: 'Audio data is written to temp files (not stdout) because binary PCM data contains 0x0A bytes that corrupt NDJSON framing.',
    },
  ],
}
