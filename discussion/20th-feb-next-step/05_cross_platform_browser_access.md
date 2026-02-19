# Cross-Platform Browser Access: How Will JARVIS Work on Windows/Linux?

> **Date**: 2026-02-19
> **Context**: The Browser Tool currently uses AppleScript to read Chrome tabs. This is macOS-only. What happens on Windows and Linux?

---

## The Problem

AppleScript is **macOS-only**. There is no equivalent on other platforms that can query Chrome's internal tab state:

- **Windows**: PowerShell/COM could talk to Internet Explorer (dead) but has no Chrome integration. AutoHotKey can read window titles but not individual tab URLs.
- **Linux**: `xdotool` and `wmctrl` can read the active window title but cannot enumerate tabs or get URLs from Chrome.

The current AppleScript approach works well for macOS but is a dead end for cross-platform support.

---

## Options Evaluated

| # | Approach | Platforms | User Setup | Reliability | Effort |
|---|----------|-----------|------------|-------------|--------|
| 1 | **AppleScript** | macOS only | None | High (on macOS) | Low |
| 2 | **Chrome DevTools Protocol (CDP)** | All | Must launch Chrome with `--remote-debugging-port=9222` | High | Medium |
| 3 | **Chrome Extension + Native Messaging** | All | Install JARVIS extension from Chrome Web Store | High | Medium-High |
| 4 | **Chrome Extension + Local HTTP** | All | Install JARVIS extension | High | Medium |
| 5 | **Per-platform scripting** | Per-platform | Varies | Fragile | High (maintenance) |

### Option 1: AppleScript (Current)

```applescript
tell application "Google Chrome"
  repeat with w in every window
    repeat with t in every tab of w
      get {URL of t, title of t}
    end repeat
  end repeat
end tell
```

- **Pro**: Zero user setup, works immediately on macOS
- **Con**: macOS-only, forever

### Option 2: Chrome DevTools Protocol (CDP)

Chrome exposes a JSON API at `http://localhost:9222/json` that lists all open tabs with URLs and titles when launched with the `--remote-debugging-port=9222` flag.

```bash
# Start Chrome with debugging
google-chrome --remote-debugging-port=9222

# Get all tabs
curl http://localhost:9222/json
# Returns: [{"id":"...", "title":"...", "url":"...", "type":"page"}, ...]
```

- **Pro**: Cross-platform, no extension needed, simple HTTP GET
- **Con**: Requires user to always launch Chrome with a special flag. Nobody does this normally. Could be automated via a launcher script, but it's awkward UX.

### Option 3: Chrome Extension + Native Messaging (Recommended Long-Term)

A small JARVIS Chrome extension uses `chrome.tabs.query({})` to get ALL tabs. It communicates with the JARVIS desktop app via Chrome's [Native Messaging API](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) — a stdin/stdout pipe between the extension and a registered native application.

```
┌─────────────────────────────┐     stdin/stdout      ┌──────────────┐
│ JARVIS Chrome Extension     │ ◄──── Native ────────► │ JARVIS App   │
│                             │      Messaging         │ (Rust backend)│
│ chrome.tabs.query({})       │                        │              │
│ → sends tab list to host    │                        │ Receives JSON│
│                             │                        │ tab list     │
└─────────────────────────────┘                        └──────────────┘
```

The extension is tiny (~50 lines):
```javascript
// background.js (service worker)
chrome.runtime.onMessageExternal.addListener((request, sender, sendResponse) => {
  if (request.action === 'listTabs') {
    chrome.tabs.query({}, (tabs) => {
      sendResponse(tabs.map(t => ({ url: t.url, title: t.title })));
    });
    return true; // async response
  }
});
```

Native messaging host manifest (registered per-OS):
- **macOS**: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.jarvis.app.json`
- **Windows**: Registry key `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.jarvis.app`
- **Linux**: `~/.config/google-chrome/NativeMessagingHosts/com.jarvis.app.json`

- **Pro**: Official Chrome API, works on all platforms, reliable, secure (Chrome enforces permissions)
- **Con**: User must install the extension. Native messaging host registration differs per OS. More moving parts.

### Option 4: Chrome Extension + Local HTTP

Similar to Option 3, but the extension POSTs tab data to a local HTTP server (`http://localhost:PORT`) running inside the JARVIS app, instead of using native messaging.

```
┌─────────────────────────────┐     HTTP POST          ┌──────────────┐
│ JARVIS Chrome Extension     │ ───── localhost ──────► │ JARVIS App   │
│                             │      :PORT/tabs         │ (Rust backend)│
│ chrome.tabs.query({})       │                        │              │
│ → POST to localhost         │                        │ HTTP server  │
│                             │                        │ receives JSON│
└─────────────────────────────┘                        └──────────────┘
```

- **Pro**: Simpler than native messaging (no host registration), cross-platform
- **Con**: Still requires extension install. Localhost HTTP is less secure. CORS handling needed. Firewall may block.

### Option 5: Per-Platform Scripting (Not Recommended)

Write different scripts for each OS:
- macOS: AppleScript
- Windows: AutoHotKey or PowerShell (can only get window titles, not tab URLs)
- Linux: xdotool (can only get window titles, not tab URLs)

- **Pro**: No extension needed
- **Con**: Windows and Linux versions can't actually get tab URLs — only the active window title. Different bugs per platform. Maintenance nightmare. Fundamentally broken.

---

## Recommendation: Phased Approach

### Phase 1 — Competition (Now → Mar 13, 2026)

**Ship with AppleScript on macOS.** It works, it's fast, zero user setup. The `BrowserAdapter` trait we've designed makes this a clean implementation detail:

```rust
// adapters/chrome_applescript.rs — macOS only
pub struct ChromeAppleScriptAdapter;

impl BrowserAdapter for ChromeAppleScriptAdapter {
    fn name(&self) -> &str { "Chrome (macOS)" }
    async fn list_tabs(&self) -> Result<Vec<RawTab>, String> {
        // osascript -e 'tell application "Google Chrome" ...'
    }
}
```

### Phase 2 — Post-Competition

**Add Chrome Extension as the universal adapter.** The same `BrowserAdapter` trait, different implementation:

```rust
// adapters/chrome_extension.rs — all platforms
pub struct ChromeExtensionAdapter {
    port: u16,  // localhost HTTP port
}

impl BrowserAdapter for ChromeExtensionAdapter {
    fn name(&self) -> &str { "Chrome (Extension)" }
    async fn list_tabs(&self) -> Result<Vec<RawTab>, String> {
        // GET http://localhost:{port}/tabs
        // or native messaging pipe
    }
}
```

The frontend doesn't change at all. The extractor pipeline doesn't change. Only the adapter swaps.

### Phase 3 — Multi-Browser

With the extension approach, supporting other Chromium browsers (Edge, Arc, Brave) is nearly free — same extension API. Firefox has a compatible `browser.tabs` API via WebExtensions.

---

## What This Means for the Architecture

The `BrowserAdapter` trait we designed in the Browser Tool plan is exactly right:

```rust
pub trait BrowserAdapter {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn list_tabs(&self) -> Result<Vec<RawTab>, String>;
}
```

This trait absorbs the cross-platform complexity. Everything downstream (classification, extraction, gisting, UI) is platform-agnostic and never changes.

```
Phase 1:  BrowserAdapter → ChromeAppleScriptAdapter (macOS)
Phase 2:  BrowserAdapter → ChromeExtensionAdapter   (all platforms)
Phase 3:  BrowserAdapter → [Chrome, Firefox, Safari, Arc...]
```

The adapter pattern was the right call.

---

## Industry Precedent

- **Rewind AI / Limitless**: Uses screen recording (OS-level), not browser APIs
- **Microsoft Recall**: Uses OS-level screen capture + OCR
- **Arc Browser**: Has native extension-like capabilities built in
- **Tab managers (SideSpace, OneTab)**: All use Chrome Extension APIs
- **Perplexity Comet**: Is itself a browser, so no adapter needed

Every tool that needs reliable access to browser tab data from an external desktop app uses a **browser extension**. There is no alternative that works cross-platform.

---

## Sources

- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
- [chrome.tabs API](https://developer.chrome.com/docs/extensions/reference/api/tabs)
- [MDN: Native Messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging)
- [WebSocket in Chrome Extension Service Workers](https://developer.chrome.com/docs/extensions/how-to/web-platform/websockets)
- [Native Messaging as Bridge (Medium)](https://medium.com/fme-developer-stories/native-messaging-as-bridge-between-web-and-desktop-d288ea28cfd7)
- [Rewind AI / Limitless Guide](https://skywork.ai/skypage/en/Rewind-AI-&-Limitless:-The-Ultimate-Guide-to-Your-Digital-Memory/1976181260991655936)
- [SideSpace Tab Manager](https://www.sidespace.app/blog/ten-best-tab-manager-in-2025)
