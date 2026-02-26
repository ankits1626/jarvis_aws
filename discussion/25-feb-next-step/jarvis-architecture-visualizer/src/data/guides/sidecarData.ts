import type { Guide } from './types'

export const sidecarGuide: Guide = {
  id: 'guide-sidecar',
  title: 'The Sidecar Pattern 101',
  subtitle: 'Build a tiny helper program that runs alongside your app — then see how Jarvis uses two of them',
  color: 'teal',
  icon: '🚗',

  sections: [
    // ── 0. What Are We Building? ──
    {
      id: 'big-picture',
      title: 'What Are We Building?',
      content: [
        {
          type: 'text',
          body: `You\'re going to build something simple: a Python script that counts words. Then you\'ll write a Rust program that starts that Python script, sends it text, and reads back the word count.\n\nTwo programs. Running at the same time. Talking to each other.\n\nThat\'s the sidecar pattern. And it\'s exactly how Jarvis works — except instead of counting words, Jarvis\'s sidecars capture microphone audio (Swift) and run AI models (Python).`,
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'idea', label: '1. The idea', icon: '💡' },
            { id: 'build', label: '2. Build a helper', icon: '🐍' },
            { id: 'spawn', label: '3. Start it from Rust', icon: '🚀' },
            { id: 'talk', label: '4. Send & receive', icon: '💬' },
            { id: 'watch', label: '5. Watch for problems', icon: '👁️' },
            { id: 'bug', label: '6. A real bug story', icon: '🐛' },
            { id: 'bundle', label: '7. Ship it together', icon: '📦' },
            { id: 'jarvis', label: '8. How Jarvis does it', icon: '🤖' },
          ],
          connections: [
            { from: 'idea', to: 'build' },
            { from: 'build', to: 'spawn' },
            { from: 'spawn', to: 'talk' },
            { from: 'talk', to: 'watch' },
            { from: 'watch', to: 'bug' },
            { from: 'bug', to: 'bundle' },
            { from: 'bundle', to: 'jarvis' },
          ],
        },
        {
          type: 'text',
          body: `By the end, you\'ll understand:\n- Why your main app sometimes needs a helper program\n- How to start one, talk to it, and shut it down\n- Why some data breaks when sent the wrong way (a real Jarvis debugging story)\n- How Jarvis bundles Swift and Python helpers inside a desktop app`,
        },
      ],
    },

    // ── 1. The Idea ──
    {
      id: 'the-idea',
      title: 'Step 1: Why Would You Run TWO Programs?',
      content: [
        {
          type: 'text',
          body: `Imagine you run a bakery. You\'re great at making bread. But one day a customer wants a wedding cake. You COULD learn cake decorating from scratch — buy new tools, practice for months. Or you could call your friend who\'s already a cake decorator and say: "Hey, I\'ll handle the bread, you handle the cake. We\'ll work side by side in my kitchen."\n\nThat friend is a **sidecar** — a separate worker who runs alongside you, doing a job you can\'t (or shouldn\'t) do yourself.\n\nIn software, the same thing happens. Your main program is great at some things but terrible at others:`,
        },
        {
          type: 'text',
          body: `**Jarvis (Rust)** is great at: managing windows, handling commands, storing data, coordinating everything.\n\n**Jarvis (Rust)** is NOT great at: capturing microphone audio on macOS (Apple designed those tools for Swift), or running AI models (the ML world runs on Python).\n\nSo Jarvis calls two friends:\n- **JarvisListen** — a Swift program that captures audio\n- **IntelligenceKit** — a Python program that runs AI models\n\nThey run side by side. Jarvis is the boss (starts them, tells them what to do, shuts them down). The helpers do their specialized jobs.`,
        },
        {
          type: 'concept-card',
          term: 'Sidecar',
          explanation: 'A separate program that runs alongside your main program. Your main program starts it, sends it work, and shuts it down when done. They\'re two independent programs cooperating.',
          example: 'Your Rust app starts a Python script. The Python script runs AI inference. Your Rust app sends text and receives summaries. When the user closes the app, Rust tells Python to shut down.',
        },
        {
          type: 'concept-card',
          term: 'Process',
          explanation: 'A program that\'s currently running. When you open Chrome, that\'s a process. When you open Spotify, that\'s another process. Each process has its own memory and runs independently. If one crashes, the others keep going.',
          example: 'If you force-quit Spotify, Chrome doesn\'t crash. They\'re separate processes. Same with sidecars — if the Python helper crashes, your Rust app stays alive.',
        },
        {
          type: 'quiz',
          question: 'Why does Jarvis use a Swift HELPER for audio capture instead of doing it all in Rust?',
          options: [
            'Rust is too slow for audio',
            'Apple built audio capture tools for Swift — using them from Rust would be extremely complex and fragile',
            'Swift is faster than Rust',
            'Tauri requires Swift for audio',
          ],
          correctIndex: 1,
          explanation: 'Apple\'s audio capture APIs (AVFoundation) are designed for Swift and Objective-C. You CAN call them from Rust, but it requires writing dangerous "unsafe" bridge code that\'s hard to maintain. A Swift sidecar lets you use Apple\'s tools the way Apple intended.',
        },
      ],
    },

    // ── 2. Build the Helper ──
    {
      id: 'build-helper',
      title: 'Step 2: Build the Helper (Python Word Counter)',
      content: [
        {
          type: 'text',
          body: `Let\'s start with the helper program. It\'s intentionally tiny — just a Python script that:\n1. Waits for a line of text\n2. Counts the words\n3. Sends back the count\n4. Repeats until told to stop\n\nCreate a file called \`word_counter.py\`:`,
        },
        {
          type: 'code',
          language: 'python',
          code: `# word_counter.py — our sidecar helper
import sys
import json

# Read one line at a time from stdin (standard input)
# "stdin" is like a mailbox — the main program drops
# messages in, we pick them up
for line in sys.stdin:
    line = line.strip()  # remove the newline character

    # If the main program says "quit", we stop
    if line == "quit":
        break

    # Parse the incoming JSON message
    try:
        request = json.loads(line)
        text = request.get("text", "")
        word_count = len(text.split())

        # Send back a JSON response on stdout (standard output)
        # "stdout" is our mailbox going the OTHER direction
        response = {"ok": True, "count": word_count}
        print(json.dumps(response), flush=True)

    except json.JSONDecodeError:
        # If we can't understand the message, say so
        error = {"ok": False, "error": "Invalid JSON"}
        print(json.dumps(error), flush=True)`,
          caption: 'FILE: word_counter.py — A tiny helper that counts words in text',
        },
        {
          type: 'text',
          body: `Let\'s test it by hand. Open a terminal and run:\n\`\`\`bash\npython3 word_counter.py\n\`\`\`\n\nNow type this and press Enter:\n\`\`\`\n{"text": "hello world how are you"}\n\`\`\`\n\nYou should see:\n\`\`\`\n{"ok": true, "count": 5}\n\`\`\`\n\nType \`quit\` to stop. Congratulations — you just built a sidecar! It sits there, waiting for work, processing it, and sending results back.`,
        },
        {
          type: 'concept-card',
          term: 'stdin (Standard Input)',
          explanation: 'A text channel where a program RECEIVES input. Think of it as the program\'s inbox. When you type in the terminal, you\'re writing to stdin. When another program spawns this one, IT writes to stdin instead of you.',
          example: 'You typed {"text": "hello world"} and pressed Enter. That text went through stdin into word_counter.py. Later, your Rust program will write to stdin instead of you typing.',
        },
        {
          type: 'concept-card',
          term: 'stdout (Standard Output)',
          explanation: 'A text channel where a program SENDS output. Think of it as the program\'s outbox. print() in Python writes to stdout. The main program reads from this channel to get responses.',
          example: 'word_counter.py used print() to send {"ok": true, "count": 5}. That went to stdout, which appeared in your terminal. Later, your Rust program will read from stdout instead of it showing on screen.',
        },
        {
          type: 'concept-card',
          term: 'JSON (JavaScript Object Notation)',
          explanation: 'A text format for structured data. Key-value pairs wrapped in curly braces. Both humans and programs can read it easily. Perfect for sending messages between programs.',
          example: '{"text": "hello world", "count": 2} — keys are "text" and "count", values are the string and the number.',
        },
        {
          type: 'text',
          body: `Notice the protocol: one JSON message per line. This format has a name — **NDJSON** (Newline-Delimited JSON). Each message is a complete JSON object, followed by a newline character (\\n). It\'s dead simple:\n- Write one JSON object\n- Add \\n at the end\n- That\'s one message\n\nJarvis\'s IntelligenceKit uses exactly this same pattern to exchange AI commands.`,
        },
        {
          type: 'concept-card',
          term: 'NDJSON (Newline-Delimited JSON)',
          explanation: 'A super simple protocol: one JSON object per line. Send a message = write JSON + press Enter. Receive a message = read one line. No headers, no framing, no complexity.',
          example: 'Line 1: {"command":"summarize","text":"..."} ← request\nLine 2: {"ok":true,"result":"..."} ← response\nEach line is one complete message.',
        },
      ],
    },

    // ── 3. Start It From Rust ──
    {
      id: 'spawn-from-rust',
      title: 'Step 3: Start the Helper from Rust',
      content: [
        {
          type: 'text',
          body: `Now comes the interesting part. Instead of YOU typing into the Python script, your Rust program will do it.\n\nThe Rust program will:\n1. **Spawn** (start) the Python script as a child process\n2. Write to its stdin (send commands)\n3. Read from its stdout (get responses)\n4. Kill it when done\n\nFirst, set up the Rust project:`,
        },
        {
          type: 'code',
          language: 'bash',
          code: `# Create a new Rust project
cargo new sidecar-demo
cd sidecar-demo

# Copy word_counter.py into this folder
cp ../word_counter.py .`,
          caption: 'Set up the project — Rust code + Python helper side by side',
        },
        {
          type: 'text',
          body: `Now write the Rust code that starts the Python script and talks to it. Open \`src/main.rs\`:`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// src/main.rs — the main program that controls the sidecar
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn main() {
    // ── Step 1: SPAWN the helper ──
    // Command::new starts a new process, like double-clicking
    // an app. We start "python3" and tell it to run our script.
    //
    // Stdio::piped() means "I want to write to its stdin and
    // read from its stdout, like plugging in pipes between us"
    let mut child = Command::new("python3")
        .arg("word_counter.py")
        .stdin(Stdio::piped())    // we'll write TO the helper
        .stdout(Stdio::piped())   // we'll read FROM the helper
        .spawn()                  // actually start it!
        .expect("Failed to start Python helper");

    // Take ownership of the stdin/stdout pipes
    let mut stdin = child.stdin.take()
        .expect("Failed to get stdin");
    let stdout = child.stdout.take()
        .expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    println!("Helper started! Sending some text...");

    // ── Step 2: TALK to the helper ──
    // Send a JSON command (write to stdin)
    let command = r#"{"text": "the quick brown fox jumps"}"#;
    writeln!(stdin, "{}", command)
        .expect("Failed to write to helper");

    // Read the JSON response (read one line from stdout)
    let mut response = String::new();
    reader.read_line(&mut response)
        .expect("Failed to read from helper");

    println!("Sent: {}", command);
    println!("Got back: {}", response.trim());

    // Send another one!
    let command2 = r#"{"text": "hello"}"#;
    writeln!(stdin, "{}", command2)
        .expect("Failed to write to helper");

    let mut response2 = String::new();
    reader.read_line(&mut response2)
        .expect("Failed to read from helper");

    println!("Sent: {}", command2);
    println!("Got back: {}", response2.trim());

    // ── Step 3: KILL the helper ──
    // Tell it to quit gracefully
    writeln!(stdin, "quit")
        .expect("Failed to send quit");

    // Wait for the process to finish
    let status = child.wait()
        .expect("Failed to wait for helper");

    println!("Helper exited with: {}", status);
}`,
          caption: 'FILE: src/main.rs — Rust program that spawns, talks to, and kills the Python helper',
        },
        {
          type: 'text',
          body: `Run it:\n\`\`\`bash\ncargo run\n\`\`\`\n\nYou should see:\n\`\`\`\nHelper started! Sending some text...\nSent: {"text": "the quick brown fox jumps"}\nGot back: {"ok": true, "count": 5}\nSent: {"text": "hello"}\nGot back: {"ok": true, "count": 1}\nHelper exited with: exit status: 0\n\`\`\`\n\nThat\'s it. Two programs, different languages, talking to each other through pipes. This is the sidecar pattern.`,
        },
        {
          type: 'concept-card',
          term: 'spawn()',
          explanation: 'Starts a new process. Like double-clicking an app — it creates a running program. The "parent" process (your Rust app) can control the "child" process (the Python script) — send it data, read its output, or kill it.',
          example: 'Command::new("python3").arg("word_counter.py").spawn() starts Python running your script. Returns a handle you can use to talk to it or stop it.',
        },
        {
          type: 'concept-card',
          term: 'Stdio::piped()',
          explanation: 'Connects your program\'s stdin/stdout to the child process with a "pipe" — like a tube between two rooms. You write into one end, the child reads from the other end. Without piped(), the child\'s output would just appear in your terminal.',
          example: 'Without piped(): Python\'s print() shows up in your terminal. With piped(): Python\'s print() gets captured by your Rust program so it can read and process the output.',
        },
        {
          type: 'concept-card',
          term: 'Child Process',
          explanation: 'A process started BY another process. The starter is the "parent," the started one is the "child." The parent can wait for the child to finish, send it signals, or kill it. If the parent exits, the child may be cleaned up by the OS.',
          example: 'Your Rust program (parent) spawns Python (child). Rust controls Python\'s lifetime. When Rust sends "quit," Python exits. If Rust crashes, the OS cleans up the orphaned Python process.',
        },
        {
          type: 'quiz',
          question: 'What does Stdio::piped() do when spawning a child process?',
          options: [
            'Makes the child process run faster',
            'Connects the parent\'s write end to the child\'s stdin, and the child\'s stdout to the parent\'s read end — like plugging pipes between them',
            'Saves the child\'s output to a file',
            'Prevents the child from printing anything',
          ],
          correctIndex: 1,
          explanation: 'piped() creates a connection between the two processes. The parent writes into one end of the pipe → data comes out the child\'s stdin. The child writes to stdout → data comes out the parent\'s read end. Two-way communication through pipes.',
        },
      ],
    },

    // ── 4. Add Error Handling ──
    {
      id: 'watch-for-problems',
      title: 'Step 4: Watch for Problems',
      content: [
        {
          type: 'text',
          body: `Our demo works, but it\'s fragile. What if the Python script crashes? What if it takes forever to respond? What if the file doesn\'t exist?\n\nA real sidecar manager handles four things that can go wrong:`,
        },
        {
          type: 'text',
          body: `**1. Spawn failure** — The helper program doesn\'t exist or can\'t start.\n\nImagine you call your cake decorator friend but their phone is disconnected. You need to handle this gracefully instead of panicking.`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// Instead of .expect() which crashes your program...
let child = Command::new("python3")
    .arg("word_counter.py")
    .spawn();

// ...handle the error gracefully
match child {
    Ok(child) => println!("Helper started!"),
    Err(e) => {
        println!("Couldn't start helper: {}", e);
        println!("Is Python installed? Is the script there?");
        return;  // exit gracefully, don't crash
    }
}`,
          caption: 'Handle the case where the helper can\'t start',
        },
        {
          type: 'text',
          body: `**2. Crash** — The helper starts but dies unexpectedly.\n\nYour cake decorator shows up but faints halfway through. You need to detect this and tell the customer.`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// After the helper finishes (or crashes), check how it exited
let status = child.wait().expect("Failed to wait");

if status.success() {
    println!("Helper finished normally");
} else {
    // Non-zero exit code = something went wrong
    println!("Helper crashed! Exit code: {:?}", status.code());
    // Tell the user something went wrong
}`,
          caption: 'Detect crashes by checking the exit code',
        },
        {
          type: 'text',
          body: `**3. Hang** — The helper is alive but stops responding. It\'s stuck.\n\nYour cake decorator is in the kitchen but staring at the wall. No cake is coming out. You can\'t wait forever.\n\nThe fix is a **timeout** — if no response comes within a certain time, give up and report an error. Jarvis uses 30-second timeouts:`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// Jarvis's approach: wrap every read with a timeout
// (This uses tokio, Rust's async runtime)

// "Give me a response within 30 seconds, or fail"
let result = tokio::time::timeout(
    Duration::from_secs(30),
    reader.read_line(&mut response)
).await;

match result {
    Ok(Ok(_)) => println!("Got response: {}", response),
    Ok(Err(e)) => println!("Read error: {}", e),
    Err(_) => println!("Helper didn't respond in 30s — it's hung!"),
}`,
          caption: 'Timeouts prevent waiting forever for a stuck helper',
        },
        {
          type: 'text',
          body: `**4. Permission error** — The helper tries to do something the OS won\'t allow.\n\nYour cake decorator tries to use your oven but you haven\'t given them the key. On macOS, this happens with microphone access, screen recording, and file access.\n\nJarvis detects this by scanning the helper\'s error output (stderr) for keywords:`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// JarvisListen (the audio sidecar) writes errors to stderr
// Jarvis reads stderr and looks for permission-related words

if error_text.contains("permission")
    || error_text.contains("Microphone")
    || error_text.contains("Screen Recording")
{
    // Show the user a friendly message:
    // "Please open System Settings > Privacy and
    //  allow Jarvis to use the microphone"
    show_permission_prompt();
}`,
          caption: 'Detect permission errors by scanning the helper\'s error output',
        },
        {
          type: 'concept-card',
          term: 'stderr (Standard Error)',
          explanation: 'A third channel (besides stdin and stdout) specifically for error messages. Programs write errors here so they don\'t get mixed up with normal output. The parent can read stderr separately to detect problems.',
          example: 'stdout says {"ok": true, "count": 5} (normal output). stderr says "Warning: low memory" (error/diagnostic output). Reading them separately means errors don\'t corrupt your data.',
        },
        {
          type: 'concept-card',
          term: 'Exit Code',
          explanation: 'A number a process returns when it finishes. 0 means "everything went fine." Any other number means "something went wrong." It\'s like a thumbs up (0) or thumbs down (non-zero).',
          example: 'Your Python script finishes normally → exit code 0. Python crashes with an exception → exit code 1. Your Rust program checks this number to know if the helper succeeded.',
        },
        {
          type: 'concept-card',
          term: 'Timeout',
          explanation: 'A maximum time you\'re willing to wait for a response. If the response doesn\'t come in time, you stop waiting and treat it as a failure. Prevents your program from freezing forever when a helper gets stuck.',
          example: 'Jarvis sends "summarize this article" to IntelligenceKit. If no response comes in 30 seconds, Jarvis stops waiting and shows an error message to the user.',
        },
        {
          type: 'quiz',
          question: 'Your sidecar is alive but hasn\'t responded for 2 minutes. What happened and what should you do?',
          options: [
            'It\'s working fine, just be patient',
            'It\'s probably hung (stuck). Use a timeout to stop waiting and report the failure.',
            'Kill your own program and restart everything',
            'Send it more requests to wake it up',
          ],
          correctIndex: 1,
          explanation: 'A process that\'s alive but not responding is "hung." The fix is timeouts — set a maximum wait time (Jarvis uses 30 seconds). When the timeout fires, stop waiting, report the error, and optionally restart the helper.',
        },
      ],
    },

    // ── 5. When NOT to Use Pipes ──
    {
      id: 'binary-bug',
      title: 'Step 5: The Bug — When Pipes Break (A Real Jarvis Story)',
      content: [
        {
          type: 'text',
          body: `Everything we\'ve built so far uses stdin/stdout pipes with text messages. JSON goes in, JSON comes out. This works perfectly.\n\nBut there\'s a trap. A real one that bit Jarvis.\n\n**The story**: JarvisListen (the Swift audio helper) captures microphone audio. The original plan was simple — have it write the raw audio bytes to stdout, and Jarvis reads them. Same pattern as our word counter, right?\n\n**The bug**: The audio was corrupted. Sometimes fine, sometimes garbled. Completely unpredictable.\n\n**It took hours to find the cause.** Here\'s what happened:`,
        },
        {
          type: 'text',
          body: `Audio data is **binary** — raw numbers representing sound waves. These numbers can be anything from 0 to 255 (for each byte).\n\nOne of those numbers is **10**. In decimal, ten. In hexadecimal, 0x0A.\n\nHere\'s the problem: the number 10 is ALSO the code for a **newline character** (\\n). When you press Enter on your keyboard, that\'s byte 10.\n\nTauri\'s shell plugin reads stdout as TEXT. It splits the stream into "lines" whenever it sees byte 10 (newline). This is perfect for JSON messages — each line is one message.\n\nBut audio data isn\'t text. The byte 10 appears constantly in audio — it\'s just a normal sound wave value. So when JarvisListen sent audio through stdout:`,
        },
        {
          type: 'code',
          language: 'text',
          code: `Audio bytes sent by JarvisListen:
[05] [12] [0A] [7F] [33] [0A] [01] [44]
              ↑                ↑
          byte 10!          byte 10!
       (sounds like        (sounds like
        quiet hum)          quiet hum)

What Tauri's shell plugin saw:
"Oh, I see newlines! Let me split into lines!"

Line 1: [05] [12]                    ← missing [0A]!
Line 2: [7F] [33]                    ← missing [0A]!
Line 3: [01] [44]

The audio bytes [0A] were EATEN by the text splitter.
The audio waveform was corrupted.`,
          caption: 'Binary byte 10 looks like a newline to text-mode readers — audio gets scrambled',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'swift', label: 'JarvisListen', icon: '🎤', color: '#f59e0b' },
            { id: 'stdout', label: 'stdout (text mode)', icon: '📤', color: '#ef4444' },
            { id: 'tauri', label: 'Tauri splits on "\\n"', icon: '✂️', color: '#ef4444' },
            { id: 'rust', label: 'Jarvis gets fragments', icon: '🦀', color: '#f59e0b' },
            { id: 'broken', label: 'Broken audio!', icon: '💥', color: '#ef4444' },
          ],
          connections: [
            { from: 'swift', to: 'stdout', label: 'raw audio bytes' },
            { from: 'stdout', to: 'tauri', label: 'splits on byte 10' },
            { from: 'tauri', to: 'rust', label: 'chunks with missing bytes' },
            { from: 'rust', to: 'broken', label: 'audio is garbled!' },
          ],
        },
        {
          type: 'text',
          body: `**The fix**: Don\'t send binary data through text pipes. Instead, JarvisListen writes audio to a **file** (specifically a FIFO — a special kind of file that works like a pipe but handles binary safely).\n\nstdout is now ONLY used for text messages (errors, warnings). Audio goes through the file where no newline-splitting happens.`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// BEFORE (broken):
// Audio sent through stdout → Tauri splits on byte 10
let (rx, child) = sidecar.spawn()?;
while let Some(CommandEvent::Stdout(chunk)) = rx.recv() {
    process_audio(chunk); // broken! bytes missing!
}

// AFTER (fixed):
// Audio written to a file, stdout only for text
let sidecar = sidecar.args(["--output", "/tmp/audio.fifo"]);
let (rx, child) = sidecar.spawn()?;

// Read audio from the file — binary-safe, no splitting
let mut file = File::open("/tmp/audio.fifo")?;
let mut buf = [0u8; 3200];
while file.read(&mut buf) > 0 {
    process_audio(buf); // works! all bytes intact!
}`,
          caption: 'The fix: binary data goes through files, text goes through stdin/stdout',
        },
        {
          type: 'concept-card',
          term: 'Binary vs Text',
          explanation: 'Text data uses characters (letters, numbers, punctuation). Binary data is raw bytes — any value from 0-255. Text pipes assume certain bytes have special meaning (like 10 = newline). Binary data doesn\'t — 10 is just the number ten. Sending binary through text pipes corrupts it.',
          example: 'JSON text: {"count": 5} ← safe through text pipes. Audio bytes: [05, 0A, 7F] ← NOT safe, because 0A gets treated as a newline.',
        },
        {
          type: 'concept-card',
          term: 'FIFO (First In First Out / Named Pipe)',
          explanation: 'A special file on disk that works like a pipe — one program writes, another reads. Data flows through it without being stored on disk. Unlike stdin/stdout, it doesn\'t do any text processing, so binary data passes through safely.',
          example: 'JarvisListen writes audio bytes to /tmp/audio.fifo. AudioRouter opens the same path and reads the bytes. Data flows through the FIFO without any modification — no newline splitting, no corruption.',
        },
        {
          type: 'text',
          body: `**The lesson**: Pick the right communication channel for your data type.\n\n- **Text data (JSON, commands)** → stdin/stdout pipes with NDJSON. Simple and reliable.\n- **Binary data (audio, images, files)** → File or FIFO. No text processing to corrupt your bytes.\n\nOur word counter uses JSON (text) → stdin/stdout is perfect.\nJarvisListen sends audio (binary) → needs a file/FIFO instead.`,
        },
        {
          type: 'quiz',
          question: 'Why does IntelligenceKit (the AI sidecar) use stdin/stdout successfully, while JarvisListen (the audio sidecar) cannot?',
          options: [
            'IntelligenceKit is written in Python, JarvisListen is in Swift',
            'IntelligenceKit sends JSON text (safe through text pipes). JarvisListen sends raw binary audio (corrupted by text pipes that split on newlines).',
            'IntelligenceKit is faster',
            'JarvisListen sends too much data',
          ],
          correctIndex: 1,
          explanation: 'It\'s about the DATA, not the language. JSON is text — newlines are actual message separators. Audio is binary — byte 10 is a sound sample, not a newline. Text pipes work for text. Binary needs a different channel.',
        },
      ],
    },

    // ── 6. How Jarvis Ships Helpers Inside the App ──
    {
      id: 'bundling',
      title: 'Step 6: Ship It — Bundling Helpers Inside Your App',
      content: [
        {
          type: 'text',
          body: `In our demo, word_counter.py sits next to our Rust code and we run it with \`python3 word_counter.py\`. That works on YOUR computer because you have Python installed.\n\nBut when you ship a desktop app to users, they might not have Python (or Swift, or whatever language your helper is written in). You need to bundle the helper INSIDE your app so it "just works."\n\nTauri solves this with a feature called **externalBin**:`,
        },
        {
          type: 'code',
          language: 'json',
          code: `// tauri.conf.json — tell Tauri what helpers to bundle
{
  "bundle": {
    "externalBin": [
      "binaries/JarvisListen",
      "binaries/IntelligenceKit"
    ]
  }
}`,
          caption: 'This config tells Tauri: "put these programs inside the app bundle"',
        },
        {
          type: 'text',
          body: `The helpers are pre-compiled into standalone executables (no Python or Swift runtime needed). They live in a \`binaries/\` folder:\n\n\`\`\`\nsrc-tauri/\n├── binaries/\n│   ├── JarvisListen-aarch64-apple-darwin    ← for M1/M2 Macs\n│   ├── JarvisListen-x86_64-apple-darwin     ← for Intel Macs\n│   ├── IntelligenceKit-aarch64-apple-darwin\n│   └── IntelligenceKit-x86_64-apple-darwin\n\`\`\`\n\nNotice the suffixes — **aarch64** (ARM chips like M1) and **x86_64** (Intel chips). Tauri automatically picks the right one for the user\'s computer.\n\nThen in Rust, instead of \`Command::new("python3")\`, you use Tauri\'s sidecar API:`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// In our demo, we did:
Command::new("python3").arg("word_counter.py").spawn()

// In Jarvis, Tauri finds the right binary automatically:
let sidecar = app_handle
    .shell()
    .sidecar("JarvisListen")  // Tauri finds the right binary
    .unwrap();

let sidecar_with_args = sidecar.args([
    "--mono",
    "--sample-rate", "16000",
    "--output", "/tmp/audio.fifo",
]);

let (rx, child) = sidecar_with_args.spawn().unwrap();
// rx = receiver for monitoring events (errors, crashes)
// child = handle to kill it later`,
          caption: 'Tauri\'s sidecar() finds the bundled binary — no "is Python installed?" worries',
        },
        {
          type: 'concept-card',
          term: 'externalBin',
          explanation: 'A Tauri config that says "bundle these executables inside my app." When the app is packaged, Tauri copies them into the .app bundle. At runtime, sidecar() finds the right binary for the user\'s CPU architecture.',
          example: 'On an M1 Mac, sidecar("JarvisListen") resolves to JarvisListen-aarch64-apple-darwin inside the app bundle. On Intel, it resolves to JarvisListen-x86_64-apple-darwin.',
        },
        {
          type: 'text',
          body: `You also need to give your app **permission** to run external programs. Tauri\'s security model requires you to explicitly allow it:\n\n\`\`\`json\n// capabilities/default.json\n{\n  "permissions": ["shell:allow-execute"]\n}\n\`\`\`\n\nWithout this, Tauri blocks sidecar spawning. This is deliberate — a desktop app shouldn\'t run arbitrary programs unless you specifically allow it.`,
        },
      ],
    },

    // ── 7. See It In Jarvis ──
    {
      id: 'in-jarvis',
      title: 'Step 7: How Jarvis Uses Two Sidecars',
      content: [
        {
          type: 'text',
          body: `Now that you\'ve built a sidecar from scratch, let\'s see how Jarvis uses the same pattern at scale. Jarvis has two sidecars, each with a different communication style:`,
        },
        {
          type: 'comparison',
          leftLabel: 'JarvisListen (Swift)',
          rightLabel: 'IntelligenceKit (Python)',
          rows: [
            { label: 'What it does', left: 'Captures microphone audio', right: 'Runs AI models (summarize, tag)' },
            { label: 'Language', left: 'Swift (Apple\'s language)', right: 'Python (ML ecosystem)' },
            { label: 'Data it sends', left: 'Raw audio bytes (binary)', right: 'JSON commands/responses (text)' },
            { label: 'Communication', left: 'Writes to a FIFO file', right: 'stdin/stdout with NDJSON' },
            { label: 'Why this channel?', left: 'Binary audio breaks text pipes', right: 'JSON text works perfectly in text pipes' },
            { label: 'Managed by', left: 'RecordingManager (recording.rs)', right: 'IntelligenceKitProvider' },
          ],
        },
        {
          type: 'text',
          body: `Here\'s the real NDJSON code from IntelligenceKit — notice how similar it is to our word counter demo:`,
        },
        {
          type: 'code',
          language: 'rust',
          code: `// intelligencekit_provider.rs — real Jarvis code
// Compare this to our word counter demo!

// What Jarvis sends (like our {"text": "..."})
#[derive(Serialize)]
struct NdjsonCommand {
    command: String,            // "summarize", "tag", etc.
    content: Option<String>,    // the text to process
    output_format: Option<String>,
}

// What IntelligenceKit replies (like our {"ok": true, "count": 5})
#[derive(Deserialize)]
struct NdjsonResponse {
    ok: bool,
    result: Option<String>,
    error: Option<String>,
}

// Send a command, get a response — same pattern as our demo!
async fn send_command(&self, cmd: NdjsonCommand)
    -> Result<NdjsonResponse, String>
{
    let json = serde_json::to_string(&cmd)?;

    // Write JSON + newline to stdin (same as our writeln!)
    let stdin = state.stdin.as_mut().unwrap();
    stdin.write_all(json.as_bytes()).await?;
    stdin.write_all(b"\\n").await?;
    stdin.flush().await?;

    // Read one line from stdout (same as our read_line)
    // BUT with a 30-second timeout (our demo didn't have this)
    let mut response_line = String::new();
    let stdout = state.stdout.as_mut().unwrap();

    tokio::time::timeout(
        Duration::from_secs(30),
        stdout.read_line(&mut response_line)
    ).await??;

    serde_json::from_str(&response_line)
}`,
          caption: 'Real Jarvis code — same spawn + write + read pattern as our demo, plus timeouts',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'ui', label: 'React UI', icon: '⚛️', color: '#3b82f6' },
            { id: 'tauri', label: 'Tauri Bridge', icon: '🌉', color: '#6366f1' },
            { id: 'rust', label: 'Rust Core', icon: '🦀', color: '#f59e0b' },
            { id: 'recorder', label: 'RecordingManager', icon: '🎙️', color: '#f59e0b' },
            { id: 'intel', label: 'IntelligenceKitProvider', icon: '🧠', color: '#f59e0b' },
            { id: 'listen', label: 'JarvisListen (Swift)', icon: '🍎', color: '#10b981' },
            { id: 'mlx', label: 'IntelligenceKit (Python)', icon: '🐍', color: '#10b981' },
          ],
          connections: [
            { from: 'ui', to: 'tauri', label: 'invoke()' },
            { from: 'tauri', to: 'rust', label: 'commands' },
            { from: 'rust', to: 'recorder', label: 'manages' },
            { from: 'rust', to: 'intel', label: 'manages' },
            { from: 'recorder', to: 'listen', label: 'spawn + FIFO' },
            { from: 'intel', to: 'mlx', label: 'spawn + NDJSON' },
          ],
        },
        {
          type: 'text',
          body: `**Four languages, one app:**\n\n- **TypeScript** (React) → what the user sees and clicks\n- **Rust** (Tauri) → the brain — coordinates everything, manages sidecars\n- **Swift** (JarvisListen) → the ears — captures audio via macOS microphone APIs\n- **Python** (IntelligenceKit) → the intelligence — runs ML models (summarize, tag, classify)\n\nThe frontend never knows that Swift and Python exist. It just calls Tauri commands like \`invoke("start_recording")\`. Rust handles starting the right sidecar, talking to it, watching for errors, and cleaning up.`,
        },
      ],
    },

    // ── 8. Try It ──
    {
      id: 'try-it',
      title: 'Step 8: Try It — Extend the Word Counter',
      content: [
        {
          type: 'text',
          body: `Let\'s extend our word counter sidecar to also count characters. Add a new command to the Python helper:`,
        },
        {
          type: 'interactive-code',
          language: 'python',
          starterCode: `# Add this to word_counter.py
# When the command is "char_count", count characters instead

request = json.loads(line)
command = request.get("command", "word_count")

if command == "word_count":
    count = len(text.split())
    response = {"ok": True, "count": count}
elif command == "______":
    count = ______
    response = {"ok": True, "______": count}
else:
    response = {"ok": False, "error": "Unknown command"}

print(json.dumps(response), flush=True)`,
          solution: `# Add this to word_counter.py
# When the command is "char_count", count characters instead

request = json.loads(line)
command = request.get("command", "word_count")

if command == "word_count":
    count = len(text.split())
    response = {"ok": True, "count": count}
elif command == "char_count":
    count = len(text)
    response = {"ok": True, "chars": count}
else:
    response = {"ok": False, "error": "Unknown command"}

print(json.dumps(response), flush=True)`,
          hint: 'The new command is "char_count". To count characters in Python, use len(text). Send back the count with a key like "chars".',
          validator: (input: string) => {
            const lower = input.toLowerCase()
            return lower.includes('char_count') && lower.includes('len(text)')
          },
        },
        {
          type: 'text',
          body: `Now write the Rust code to send the new command:`,
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: `// Send a char_count command from Rust
let command = r#"{"command": "______", "text": "hello world"}"#;
writeln!(stdin, "{}", command)?;

let mut response = String::new();
reader.read_line(&mut response)?;
println!("Character count: {}", response.trim());`,
          solution: `// Send a char_count command from Rust
let command = r#"{"command": "char_count", "text": "hello world"}"#;
writeln!(stdin, "{}", command)?;

let mut response = String::new();
reader.read_line(&mut response)?;
println!("Character count: {}", response.trim());
// Output: {"ok": true, "chars": 11}`,
          hint: 'Fill in the command name to match what you added in the Python script — "char_count".',
          validator: (input: string) => {
            return input.includes('char_count')
          },
        },
        {
          type: 'text',
          body: `**What you just learned:**\n\n1. A sidecar is just a helper program that runs alongside your main app\n2. You start it with \`spawn()\`, talk to it through stdin/stdout, and stop it when done\n3. Use **NDJSON** (JSON lines) for text-based communication between processes\n4. Use **files/FIFOs** for binary data — never send binary through text pipes\n5. Always handle failures: spawn errors, crashes, hangs (timeouts), and permission issues\n6. Bundle helpers inside your app with Tauri\'s \`externalBin\` so users don\'t need to install anything\n\nYou\'ve now built the same pattern Jarvis uses to coordinate Swift audio capture and Python AI inference — from scratch.`,
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'RecordingManager (sidecar lifecycle)',
      file: 'jarvis-app/src-tauri/src/recording.rs',
      description: 'Manages JarvisListen sidecar: spawn with args, monitor stderr for errors, detect crashes, SIGTERM shutdown',
    },
    {
      concept: 'IntelligenceKit Provider (NDJSON)',
      file: 'jarvis-app/src-tauri/src/intelligence/intelligencekit_provider.rs',
      description: 'Same spawn + stdin/stdout pattern as our demo, but with 30s timeouts and async/await',
    },
    {
      concept: 'Sidecar config (externalBin)',
      file: 'jarvis-app/src-tauri/tauri.conf.json',
      description: 'Declares JarvisListen and IntelligenceKit as bundled external binaries with architecture suffixes',
    },
    {
      concept: 'Audio routing (binary-safe)',
      file: 'jarvis-app/src-tauri/src/transcription/audio_router.rs',
      description: 'Creates a FIFO for binary audio data — the fix for the stdout corruption bug',
    },
    {
      concept: 'MLX Provider (Python sidecar)',
      file: 'jarvis-app/src-tauri/src/intelligence/mlx_provider.rs',
      description: 'Another Python sidecar using the same spawn + communicate + shutdown pattern',
    },
  ],
}
