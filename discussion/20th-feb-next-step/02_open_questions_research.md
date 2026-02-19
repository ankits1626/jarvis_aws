# Open Questions — R&D Findings

## Q1: Notification Click Action

**Can tauri-plugin-notification v2 handle click actions?**

**Version**: v2.3.3 (resolved in Cargo.lock)

**Findings**:
- The plugin supports **action buttons** on notifications (macOS native support)
- When user clicks a notification or an action button, the plugin can emit events back to the app
- Bringing the app to foreground is NOT automatic — requires explicit code using Tauri's window API:
  ```rust
  if let Some(window) = app.get_webview_window("main") {
      let _ = window.set_focus();   // bring to foreground
      let _ = window.unminimize();  // in case it was minimized
  }
  ```
- The `AppHandle` is already available everywhere in our codebase (observer, commands, etc.)

**Verdict**: Yes, we can do this. When notification is clicked → bring app to foreground → emit event to open YouTube section.

---

## Q2: Quick Title Fetch

**Should we use the full scraper or a lightweight fetch for the notification?**

**Current full scraper timing**:
- Downloads entire HTML page: ~1-2 MB
- Parses ytInitialPlayerResponse JSON via brace-counting: CPU intensive
- Total: **3-8 seconds** — too slow for a notification

**YouTube oEmbed API** (lightweight alternative):
```
GET https://www.youtube.com/oembed?url={VIDEO_URL}&format=json
```

Returns ~1-2 KB JSON:
```json
{
  "title": "Video Title",
  "author_name": "Channel Name",
  "thumbnail_url": "https://i.ytimg.com/vi/{id}/hqdefault.jpg"
}
```

**Timing**: ~200-600ms — well within 1-second target.

**Two-tier approach**:
1. **Quick fetch** (oEmbed) — called by observer for notification title. Fast, <1s.
2. **Full scrape** (existing) — called when user clicks "Prepare Gist". Slow but detailed (description, duration).

**Edge cases**: oEmbed may fail for age-restricted, private, or deleted videos. Fallback: show notification without title ("New YouTube video detected").

**Verdict**: Use oEmbed API for notifications. Keep full scraper for gists. Best of both worlds.

---

## Q3: Auto-Gist

**Should JARVIS auto-prepare the gist when the user responds to the notification?**

**Consideration**: The full gist scrape takes 3-8 seconds. Two options:

- **Option A — Auto-prepare**: When user clicks notification, immediately start fetching the full gist in the background. By the time user sees the YouTube section, gist might already be ready. Risk: wasted bandwidth if user didn't actually want the gist.

- **Option B — Manual "Prepare Gist"**: Show the video card with title (from oEmbed) and a "Prepare Gist" button. User clicks when ready. Simpler, no wasted work.

- **Option C — Hybrid**: Start auto-fetching when notification is clicked, but show a loading indicator. If it finishes before user dismisses, great. If user dismisses, cancel.

**Verdict**: Start with Option B (manual). It's simpler and the user explicitly asked "would you like me to keep a gist?" implying they want to choose. Can upgrade to Option C later.

---

## Q4: Multiple Videos / Debouncing

**If user opens 5 YouTube videos quickly, do we notify for each?**

**Current debouncing**:
- Polls every 3 seconds
- Compares full URL string (`last_url`) — only fires when URL changes
- **No video ID history** — if user leaves and returns to same video, it re-fires
- Same video with different params (`&t=30s` vs `&t=60s`) triggers duplicate events

**Problems to fix**:
1. Need video-ID-based deduplication, not full URL comparison
2. Need a "seen videos" set to avoid re-notifying for the same video in one session
3. Rapid tab switching (5 videos in 15 seconds) will fire 5 notifications — could be annoying

**Proposed solution**:
- Extract video ID from URL before comparing (already have `detect_youtube()`)
- Maintain a `HashSet<String>` of seen video IDs in the polling loop
- Only notify once per video ID per session
- Clear the set when observer stops/restarts

**Verdict**: Add video-ID-based deduplication with a seen-set. One notification per unique video per session.

---

## Q5: Observer Always-On vs Toggle

**Should the observer always run, or should users be able to disable it?**

**Existing settings infrastructure**:
- Settings persist to `~/.jarvis/settings.json`
- Currently only has `TranscriptionSettings`
- Full validation + persistence framework already in place (`SettingsManager`)
- Adding `BrowserSettings { observer_enabled: bool }` is straightforward

**Auto-start pattern**:
- `ShortcutManager` already auto-starts in `setup()` — follow the same pattern
- Check settings on startup, auto-start if enabled
- Graceful if Chrome not running (observer retries every 3s, doesn't crash)

**Proposed approach**:
- Observer auto-starts on app launch (default: enabled)
- Toggle in Settings panel to disable/enable
- When disabled, stop polling and don't send notifications
- Setting persists across restarts

**Verdict**: Auto-start by default. Add a simple toggle in Settings for users who don't want it. Follow ShortcutManager pattern.

---

## Summary Table

| Question | Answer | Complexity |
|----------|--------|------------|
| Q1: Notification click → foreground | Yes, use `window.set_focus()` + event | Low |
| Q2: Quick title fetch | oEmbed API (~200-600ms) | Low |
| Q3: Auto-gist | No, manual "Prepare Gist" button | N/A (keep current) |
| Q4: Multiple videos | Video-ID dedup with HashSet | Medium |
| Q5: Always-on toggle | Auto-start + Settings toggle | Medium |
