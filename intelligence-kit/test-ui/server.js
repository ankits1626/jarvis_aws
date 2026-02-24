// IntelligenceKit Test Bridge Server
// Spawns IntelligenceKit as a child process and exposes HTTP endpoints.
// Usage: node server.js
// Then open http://localhost:3847 in your browser.

const { spawn } = require("child_process");
const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 3847;
const BINARY = path.resolve(__dirname, "../.build/debug/IntelligenceKit");

let ikProcess = null;
let pendingCallbacks = [];
let stderrLog = [];

function startIntelligenceKit() {
  if (ikProcess) {
    console.log("[bridge] IntelligenceKit already running");
    return;
  }

  console.log(`[bridge] Starting IntelligenceKit: ${BINARY}`);
  ikProcess = spawn(BINARY, [], {
    stdio: ["pipe", "pipe", "pipe"],
  });

  ikProcess.stdout.on("data", (data) => {
    const lines = data.toString().split("\n").filter(Boolean);
    for (const line of lines) {
      console.log(`[ik:stdout] ${line}`);
      if (pendingCallbacks.length > 0) {
        const cb = pendingCallbacks.shift();
        cb(null, line);
      }
    }
  });

  ikProcess.stderr.on("data", (data) => {
    const msg = data.toString().trim();
    console.log(`[ik:stderr] ${msg}`);
    stderrLog.push({ time: new Date().toISOString(), message: msg });
    // Keep last 100 log entries
    if (stderrLog.length > 100) stderrLog.shift();
  });

  ikProcess.on("close", (code) => {
    console.log(`[bridge] IntelligenceKit exited with code ${code}`);
    ikProcess = null;
    // Reject any pending callbacks
    while (pendingCallbacks.length > 0) {
      const cb = pendingCallbacks.shift();
      cb(new Error("IntelligenceKit process exited"));
    }
  });

  ikProcess.on("error", (err) => {
    console.error(`[bridge] Failed to start IntelligenceKit: ${err.message}`);
    ikProcess = null;
  });
}

function sendCommand(jsonStr) {
  return new Promise((resolve, reject) => {
    if (!ikProcess) {
      reject(new Error("IntelligenceKit not running. Start it first."));
      return;
    }

    const timeout = setTimeout(() => {
      const idx = pendingCallbacks.indexOf(cb);
      if (idx !== -1) pendingCallbacks.splice(idx, 1);
      reject(new Error("Timeout waiting for response (30s)"));
    }, 30000);

    const cb = (err, data) => {
      clearTimeout(timeout);
      if (err) reject(err);
      else resolve(data);
    };

    pendingCallbacks.push(cb);

    try {
      ikProcess.stdin.write(jsonStr + "\n");
    } catch (e) {
      clearTimeout(timeout);
      pendingCallbacks.pop();
      reject(new Error(`Failed to write to stdin: ${e.message}`));
    }
  });
}

const server = http.createServer(async (req, res) => {
  // CORS headers
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");

  if (req.method === "OPTIONS") {
    res.writeHead(200);
    res.end();
    return;
  }

  // Serve the HTML UI
  if (req.method === "GET" && (req.url === "/" || req.url === "/index.html")) {
    const htmlPath = path.join(__dirname, "index.html");
    const html = fs.readFileSync(htmlPath, "utf-8");
    res.writeHead(200, { "Content-Type": "text/html" });
    res.end(html);
    return;
  }

  // API: Start IntelligenceKit
  if (req.method === "POST" && req.url === "/api/start") {
    if (ikProcess) {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ ok: true, message: "Already running" }));
    } else {
      startIntelligenceKit();
      // Wait a moment for the process to start
      await new Promise((r) => setTimeout(r, 500));
      const running = ikProcess !== null;
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ ok: running, message: running ? "Started" : "Failed to start" }));
    }
    return;
  }

  // API: Stop IntelligenceKit
  if (req.method === "POST" && req.url === "/api/stop") {
    if (ikProcess) {
      try {
        await sendCommand('{"command":"shutdown"}');
      } catch (_) {
        // Process may have already exited
      }
      if (ikProcess) {
        ikProcess.kill("SIGTERM");
        ikProcess = null;
      }
    }
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ ok: true }));
    return;
  }

  // API: Send command
  if (req.method === "POST" && req.url === "/api/send") {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", async () => {
      try {
        const result = await sendCommand(body);
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(result);
      } catch (err) {
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ ok: false, error: err.message }));
      }
    });
    return;
  }

  // API: Get status
  if (req.method === "GET" && req.url === "/api/status") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ running: ikProcess !== null }));
    return;
  }

  // API: Get stderr logs
  if (req.method === "GET" && req.url === "/api/logs") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(stderrLog));
    return;
  }

  res.writeHead(404);
  res.end("Not found");
});

server.listen(PORT, () => {
  console.log(`[bridge] Test UI server running at http://localhost:${PORT}`);
  console.log(`[bridge] IntelligenceKit binary: ${BINARY}`);
  console.log("[bridge] Open the URL above in your browser to start testing.");
});

// Cleanup on exit
process.on("SIGINT", () => {
  console.log("\n[bridge] Shutting down...");
  if (ikProcess) ikProcess.kill("SIGTERM");
  server.close();
  process.exit(0);
});
