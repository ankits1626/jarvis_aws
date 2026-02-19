# Out-of-the-Box Browser Access: Deep Research

> **Date**: 2026-02-19
> **Context**: AppleScript is macOS-only. Chrome Extension requires user install. What creative, zero-setup approaches exist for reading browser tabs cross-platform?

---

## The 6 Approaches (Ranked by Creativity)

---

### 1. Read Chrome's Session Files Directly From Disk (Best Out-of-the-Box)

**How**: Chrome writes its open tab state to local files in the user's profile directory. These are SNSS format files that contain every open tab's URL, title, and navigation history. We can just... read them.

**File locations**:
| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/Google/Chrome/Default/Sessions/` |
| Windows | `%LocalAppData%\Google\Chrome\User Data\Default\Sessions\` |
| Linux | `~/.config/google-chrome/Default/Sessions/` |

**Existing tools that prove this works**:
- [`chrome-session-dump`](https://github.com/lemnos/chrome-session-dump) (Go) — runs `chrome-session-dump` and gets all open tab URLs from a running Chrome
- [`snss`](https://github.com/thanadolps/snss) (Rust!) — Rust crate that parses SNSS files, returns tab id, title, URL
- [`ccl-ssns`](https://github.com/cclgroupltd/ccl-ssns) (Python) — forensic-grade SNSS parser

**Rust example using the `snss` crate**:
```rust
let data = std::fs::read(session_file_path)?;
let snss = snss::parse(&data)?;
for command in snss.commands {
    if let snss::CommandContent::Tab { id, title, url, .. } = command.content {
        println!("Tab {}: {} - {}", id, title, url);
    }
}
```

**Pros**:
- Zero user setup — no extension, no flags, no permissions
- Cross-platform — same format on macOS, Windows, Linux
- Rust crate already exists (`snss`)
- No network calls — pure file read
- Works even if Chrome is the background

**Cons**:
- Chrome doesn't flush to disk instantly — there can be a few seconds of lag
- Chrome 86+ changed file naming (from `Current Session` to timestamped files in `Sessions/` folder)
- Incognito tabs are NOT written to disk (by design)
- File is locked while Chrome writes — need to handle concurrent access
- Format is undocumented (reverse-engineered), could change in Chrome updates

**Verdict**: This is the most creative zero-setup approach. The `snss` Rust crate makes it practical. Main risk is Chrome changing the format, but it's been stable for years and forensic tools depend on it.

---

### 2. OS Accessibility APIs (Read Chrome's UI Tree)

**How**: Every OS exposes an accessibility API that lets external apps read the UI tree of any application. Chrome implements accessibility, so we can walk Chrome's UI tree to find tabs and URLs.

**Per-platform**:

| OS | API | Library | What You Can Read |
|----|-----|---------|-------------------|
| macOS | AXUIElement / NSAccessibility | [`accessibility-sys`](https://docs.rs/accessibility-sys/latest/accessibility_sys/) (Rust) | Tab titles, address bar URL, window structure |
| Windows | UI Automation (UIA) | [`uiautomation-rs`](https://sourceforge.net/projects/rust-windows-uiautomat.mirror/) (Rust), [`windows`](https://microsoft.github.io/windows-docs-rs/doc/windows/UI/UIAutomation/index.html) crate | Tab strip elements, address bar text, tab titles |
| Linux | AT-SPI2 via D-Bus | `atspi` crate or D-Bus calls | Tab roles (`ATSPI_ROLE_PAGE_TAB`), titles |

**macOS approach**:
```
AXUIElementCreateApplication(chrome_pid)
  → Walk children to find tab bar (AXTabGroup)
  → Each tab has AXTitle attribute
  → Find address bar (AXTextField with "Address and search bar" description)
  → Read AXValue for current URL
```

**Windows approach** (proven by [AutoHotkey community](https://www.autohotkey.com/boards/viewtopic.php?t=104999)):
```
1. Get Chrome window via Process.GetProcessesByName("chrome")
2. Find "Address and search bar" element via UIA TreeWalker
3. Read Value property → current URL
4. Find tab strip parent, enumerate TabItem children → all tab titles
```

**Linux approach**:
```
Chrome with --force-renderer-accessibility flag
  → AT-SPI2 exposes tab tree via D-Bus
  → Walk ATSPI_ROLE_PAGE_TAB_LIST → children are tabs
```

**Pros**:
- No extension needed
- Works on all platforms (with platform-specific code)
- Can read MORE than just tabs — window position, focused element, etc.
- Existing Rust crates for each platform's accessibility API

**Cons**:
- macOS requires Accessibility permission (System Settings > Privacy > Accessibility)
- Each platform needs different code (but fits our BrowserAdapter trait!)
- Can only read the **active tab's URL** via address bar — other tabs only expose titles, not URLs
- Fragile — Chrome UI structure changes between versions
- Linux requires Chrome to be launched with `--force-renderer-accessibility`
- Performance overhead (each UIA call is cross-process)

**Key limitation**: Accessibility APIs can read **all tab titles** but typically only the **active tab's URL** (from the address bar). You can't get URLs of background tabs this way. This is the dealbreaker for our use case unless we combine it with approach #1.

**Verdict**: Great for getting the active tab URL + all tab titles. But incomplete for our "gist any tab" use case since we need URLs for all tabs. Best as a **complement** to the session file approach.

---

### 3. Screen Capture + OCR (The Recall/Rewind Approach)

**How**: Capture a screenshot of Chrome's tab bar and address bar, then use OCR to extract text.

**Technologies**:
| OS | Capture | OCR Engine |
|----|---------|------------|
| macOS | `CGWindowListCreateImage` | Apple Vision Framework (99.7% accuracy, offline) |
| Windows | `BitBlt` / Desktop Duplication API | Windows.Media.Ocr or Tesseract |
| Linux | X11 `XGetImage` / Wayland `wlr-screencopy` | Tesseract 5.0+ |

**macOS advantage**: Apple's Vision framework does OCR locally with near-perfect accuracy. Tools like [NormCap](https://github.com/dynobo/normcap) and [Screenotate](https://screenotate.com/) prove this works well.

**Pros**:
- Works with ANY browser — Chrome, Firefox, Safari, Arc, anything with pixels
- No extension, no API, no file parsing
- Can capture visual context beyond just URLs (page content visible in tab)
- Privacy-preserving if OCR runs locally

**Cons**:
- Can only read what's VISIBLE — if 50 tabs are open, only ~10 tab titles are visible
- URL only visible for the active tab (address bar)
- OCR accuracy varies with font size, scaling, DPI
- Requires screen recording/capture permission on macOS
- Heavy — screenshot + OCR is slower than file read or API call
- Fragile — depends on Chrome's visual layout

**Verdict**: Too limited for our tab listing use case (can't see all tabs). But interesting for a future "screen awareness" feature where JARVIS understands what you're LOOKING at, not just what tabs are open.

---

### 4. Chrome DevTools Protocol (CDP) via Auto-Launch

**How**: JARVIS automatically manages Chrome's launch with `--remote-debugging-port=9222`, then queries `http://localhost:9222/json` for all tabs.

**The twist**: Instead of asking the user to launch Chrome with a flag, JARVIS could:
- Detect Chrome's running state
- Kill and relaunch Chrome with the debug flag (too aggressive)
- OR: Create a "JARVIS Chrome" shortcut that launches Chrome with the flag
- OR: Modify Chrome's `.desktop` file (Linux) or shortcut (Windows) to include the flag

**API once connected**:
```bash
curl http://localhost:9222/json
# Returns:
[
  {"id": "abc", "title": "GitHub", "url": "https://github.com", "type": "page"},
  {"id": "def", "title": "YouTube", "url": "https://youtube.com/watch?v=...", "type": "page"}
]
```

Can also use [Playwright's `connectOverCDP`](https://playwright.dev/docs/api/class-browsertype) to attach to the running browser and get full programmatic control.

**Pros**:
- Full access to ALL tabs with URLs and titles
- Cross-platform (CDP works everywhere Chrome does)
- Can also interact with pages (navigate, execute JS, take screenshots)
- Well-documented, stable protocol

**Cons**:
- Requires Chrome to be started with `--remote-debugging-port`
- Security risk — anyone on localhost can access all tabs
- Modifying Chrome's launch is OS-specific and invasive
- Users may resist "JARVIS controlling my browser"

**Verdict**: Powerful but the launch-flag requirement is UX friction. Best as an opt-in power-user feature.

---

### 5. Hybrid: Session Files + Accessibility API (Recommended Future Architecture)

**How**: Combine approach #1 (session files for all tab URLs) with approach #2 (accessibility API for real-time active tab context).

```
┌─────────────────────────────────────┐
│ BrowserAdapter (Hybrid)              │
│                                      │
│  Session Files → All tab URLs+titles │
│  (cross-platform, ~2s lag)           │
│         +                            │
│  Accessibility API → Active tab URL  │
│  (real-time, current focus)          │
│         =                            │
│  Complete picture with zero setup    │
└─────────────────────────────────────┘
```

**Why this is the best long-term approach**:
- Session files give us the FULL tab list (all URLs, all titles)
- Accessibility API gives us REAL-TIME active tab (no lag)
- Neither requires a Chrome extension
- Neither requires Chrome launch flags
- Both work cross-platform (with platform-specific implementations)
- The `BrowserAdapter` trait handles the abstraction

**Per-platform implementation**:

| Component | macOS | Windows | Linux |
|-----------|-------|---------|-------|
| Session files | `~/Library/Application Support/Google/Chrome/Default/Sessions/` | `%LocalAppData%\...\Sessions\` | `~/.config/google-chrome/.../Sessions/` |
| Active tab | AppleScript OR AXUIElement | UI Automation | AT-SPI2 via D-Bus |
| Rust crates | `snss` + `accessibility-sys` | `snss` + `windows` crate | `snss` + `atspi` |

---

### 6. Wild Card: Intercept Chrome's Network via Local Proxy

**How**: JARVIS runs a local proxy (e.g., mitmproxy-style). Chrome is configured to route through `localhost:PORT`. JARVIS sees every URL Chrome requests.

**Pros**: Sees ALL navigation in real-time, not just current state
**Cons**: Requires proxy configuration, breaks HTTPS unless you install a CA cert, massive privacy/security concern

**Verdict**: Too invasive. Interesting for research but not practical for a user-facing product.

---

## Comparison Matrix

| Approach | All Tab URLs | All Tab Titles | Real-time | Zero Setup | Cross-Platform | Reliability |
|----------|-------------|----------------|-----------|------------|----------------|-------------|
| 1. Session Files | Yes | Yes | ~2s lag | Yes | Yes | Medium (format may change) |
| 2. Accessibility API | Active only | Yes | Yes | Needs permission | Yes (different code) | Medium (UI changes) |
| 3. Screen + OCR | Visible only | Visible only | Yes | Needs permission | Yes | Low |
| 4. CDP | Yes | Yes | Yes | Needs flag | Yes | High |
| 5. Hybrid (1+2) | Yes | Yes | Yes | Needs permission | Yes | Medium-High |
| 6. Chrome Extension | Yes | Yes | Yes | Install ext | Yes | High |

---

## Recommended Roadmap

```
Phase 1 (MVP — Now)
  └── AppleScript on macOS
       Simple, works, zero friction for competition demo

Phase 2 (Post-MVP)
  └── Session File Reader (snss crate)
       Cross-platform, zero user setup
       Handles the "all tabs" use case
       + AppleScript/Accessibility for real-time active tab

Phase 3 (Production)
  └── Chrome Extension (optional upgrade)
       For users who want real-time sync
       Richer features (tab events, page content access)
       + Session files as fallback when extension not installed

Phase 4 (Multi-browser)
  └── Browser-specific adapters
       Safari: AppleScript (macOS), session files
       Firefox: places.sqlite + recovery.jsonlz4
       Arc: Chrome-based (same session files)
       Edge: Chrome-based (same session files)
```

**Key insight**: Session files are the **universal backdoor**. Every Chromium browser (Chrome, Edge, Brave, Arc, Opera, Vivaldi) uses the same SNSS format. Firefox uses `recovery.jsonlz4` (also parseable). This one approach covers 95%+ of browsers.

---

## Firefox Session Files (Bonus)

Firefox stores its session in `recovery.jsonlz4` (LZ4-compressed JSON):
- macOS: `~/Library/Application Support/Firefox/Profiles/<profile>/sessionstore-backups/recovery.jsonlz4`
- Windows: `%AppData%\Mozilla\Firefox\Profiles\<profile>\sessionstore-backups\recovery.jsonlz4`
- Linux: `~/.mozilla/firefox/<profile>/sessionstore-backups/recovery.jsonlz4`

Decompress with LZ4, parse JSON → `windows[].tabs[].entries[].url` gives you every tab URL.

This means **Session files approach covers Chrome + Firefox** with zero user setup.

---

## Sources

- [Chrome Session Files — SNSS Format](https://digitalinvestigation.wordpress.com/2012/09/03/chrome-session-and-tabs-files-and-the-puzzle-of-the-pickle/)
- [`snss` — Rust SNSS Parser](https://github.com/thanadolps/snss)
- [`chrome-session-dump` — Go SNSS Tool](https://github.com/lemnos/chrome-session-dump)
- [`ccl-ssns` — Python Forensic Parser](https://github.com/cclgroupltd/ccl-ssns)
- [Chrome Session Viewer (Web)](https://github.com/lachlanallison/chrome-session-viewer)
- [Chrome Data Storage and Session Recovery](https://www.cyberengage.org/post/understanding-chrome-s-data-storage-and-session-recovery-what-your-browser-remembers)
- [macOS AXorcist — Swift Accessibility Wrapper](https://github.com/steipete/AXorcist)
- [`accessibility-sys` — Rust Accessibility Bindings](https://docs.rs/accessibility-sys/latest/accessibility_sys/)
- [Windows UI Automation + Chrome (AutoHotkey)](https://www.autohotkey.com/boards/viewtopic.php?t=104999)
- [Windows UIA Tree Walk (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-howto-walk-uiautomation-tree)
- [`windows` Rust Crate — UIAutomation](https://microsoft.github.io/windows-docs-rs/doc/windows/UI/UIAutomation/index.html)
- [AT-SPI2 (Linux Accessibility)](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [Chromium Accessibility Architecture](https://www.chromium.org/developers/design-documents/accessibility/)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [Playwright connectOverCDP](https://playwright.dev/docs/api/class-browsertype)
- [NormCap — OCR Screen Capture](https://github.com/dynobo/normcap)
- [Apple Vision Framework OCR](https://discourse.devontechnologies.com/t/apple-live-text-vision-framework-ocr/69414)
- [Chrome History & SQLite (Forensics)](https://www.foxtonforensics.com/browser-history-examiner/chrome-history-location)
- [Get All Chrome Tab URLs via UIA (BlueLightDev)](https://www.bluelightdev.com/get-list-open-chrome-tabs)
