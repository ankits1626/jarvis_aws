# Gmail Email Thread Extractor — Implementation Plan

## Context

JARVIS's Browser Tool can extract gists from YouTube, Medium, and generic pages. Next: Gmail support. When a user is viewing an email thread in Chrome, "Prepare Gist" captures the full thread — subject, participants, and all emails.

Gmail is a complex SPA with obfuscated class names. We use **stable selectors only** (ARIA roles, `data-message-id`, `[email]` attributes) and the existing `execute_js_in_tab` Chrome adapter for DOM extraction.

---

## Architecture

```
User views email thread in Gmail (Chrome)
  → BrowserTool lists tabs → Gmail tab shows [Email] badge
  → User clicks "Prepare Gist"
  → Router: SourceType::Email → gmail::extract()
  → execute_js_in_tab(url, EXTRACT_JS)
  → JS extracts: subject, sender, participants, email count, thread text
  → Returns JSON → Rust deserializes → PageGist
  → Frontend renders full thread with export support
```

Same pattern as Medium extractor — single JS call, JSON response, no new dependencies.

---

## Files to Change (6 files)

### 1. NEW: `src-tauri/src/browser/extractors/gmail.rs`

Gmail extractor module following medium.rs pattern.

**JavaScript extraction strategy** (stable selectors only):

| Data | Selector / Source | Notes |
|------|-------------------|-------|
| Subject | `document.title` | Format: "Subject - sender@email.com - Gmail" |
| Sender | Title's second-to-last segment | Always present when viewing a thread |
| Participants | `[email]` attributes on spans in `[role="main"]` | Gmail uses `email` attr on sender chips |
| Email count | Count of `[data-message-id]` elements | Stable Gmail attribute, one per email |
| Thread text | `innerText` of each `[data-message-id]` element | Separated by `--- Email N ---` headers |
| Is thread? | `email_count > 0` | Guard against inbox/folder views |

**EXTRACT_JS constant:**
```javascript
(function(){
  var d = {};

  // Subject from title: "Subject - sender@email.com - Gmail"
  var rawTitle = document.title || '';
  var titleParts = rawTitle.split(' - ');
  if (titleParts.length >= 2) {
    titleParts.pop(); // remove "Gmail"
    d.sender = (titleParts.pop() || '').trim();
    d.subject = titleParts.join(' - ');
  } else {
    d.subject = rawTitle;
    d.sender = null;
  }

  // Email count and thread text from [data-message-id] elements
  var main = document.querySelector('[role="main"]');
  var threadParts = [];
  var emailCount = 0;

  if (main) {
    var messages = main.querySelectorAll('[data-message-id]');
    emailCount = messages.length;
    for (var i = 0; i < messages.length; i++) {
      var text = messages[i].innerText;
      if (text && text.trim().length > 0) {
        threadParts.push('--- Email ' + (i + 1) + ' ---\n' + text.trim());
      }
    }
  }

  d.email_count = emailCount;
  d.thread_text = threadParts.join('\n\n');

  // Participants from [email] attributes
  var emailAttrs = [];
  if (main) {
    var spans = main.querySelectorAll('[email]');
    for (var j = 0; j < spans.length; j++) {
      var addr = spans[j].getAttribute('email');
      if (addr && emailAttrs.indexOf(addr) === -1) {
        emailAttrs.push(addr);
      }
    }
  }
  d.participants = emailAttrs;
  d.is_thread = emailCount > 0;

  // 50K char guard to avoid oversized AppleScript output
  if (d.thread_text && d.thread_text.length > 50000) {
    d.thread_text = d.thread_text.substring(0, 50000) + '\n\n[thread truncated]';
  }

  return JSON.stringify(d);
})()
```

**Rust struct:**
```rust
#[derive(Deserialize)]
struct GmailDomData {
    subject: Option<String>,
    sender: Option<String>,
    participants: Option<Vec<String>>,
    email_count: Option<u32>,
    thread_text: Option<String>,
    is_thread: Option<bool>,
}
```

**PageGist mapping:**

| PageGist field | Source |
|----------------|--------|
| `title` | `subject` (or "Unknown Thread") |
| `author` | `sender` from title |
| `description` | "3 emails · alice@x.com, bob@y.com" |
| `content_excerpt` | Full thread text (no truncation, same as Medium) |
| `published_date` | None |
| `image_url` | None |
| `extra` | `{ "email_count": 3, "participants": ["alice@x.com", "bob@y.com"] }` |

**Guard:** If `is_thread == false`, return error: "No email thread found — please open a specific email thread in Gmail"

---

### 2. MODIFY: `src-tauri/src/browser/tabs.rs`

Add `Email` to SourceType enum:
```rust
pub enum SourceType {
    YouTube,
    Article,
    Code,
    Docs,
    Email,  // ← NEW
    QA,
    News,
    Research,
    Social,
    Other,
}
```

Add classification rule in `classify_url()`:
```rust
// Email
if domain.contains("mail.google.com") {
    return SourceType::Email;
}
```

---

### 3. MODIFY: `src-tauri/src/browser/extractors/mod.rs`

Add module + routing:
```rust
pub mod generic;
pub mod gmail;    // ← NEW
pub mod medium;

// In prepare_gist():
match source_type {
    SourceType::YouTube => youtube_gist(url, &domain).await,
    SourceType::Email => gmail::extract(url, source_type, &domain).await,  // ← NEW
    _ if domain.contains("medium.com") => medium::extract(url, source_type, &domain).await,
    _ => generic::extract(url, source_type, &domain).await,
}
```

---

### 4. MODIFY: `src/state/types.ts`

```typescript
export type SourceType = 'YouTube' | 'Article' | 'Code' | 'Docs' | 'Email' | 'QA' | 'News' | 'Research' | 'Social' | 'Other';
```

---

### 5. MODIFY: `src/components/BrowserTool.tsx`

Add to SOURCE_BADGES:
```typescript
Email: { label: 'Email', className: 'source-badge email' },
```

---

### 6. MODIFY: `src/App.css`

```css
.source-badge.email {
  background: #e8f0fe;
  color: #1a73e8;
}
```

---

## Implementation Order

1. `tabs.rs` — Add `Email` variant + classify. Compiler flags all unhandled match arms.
2. `gmail.rs` — Create extractor file.
3. `extractors/mod.rs` — Register module + routing arm.
4. `types.ts` — Add `'Email'` to union. TypeScript flags missing badge.
5. `BrowserTool.tsx` — Add Email badge.
6. `App.css` — Badge style.

## Verification

1. `cargo check` — compiles cleanly
2. `cargo test` — all tests pass
3. Open Gmail → navigate to an email thread → BrowserTool → tab shows `[Email]` badge → Prepare Gist → full thread with subject, participants, all emails
4. Export button saves thread to `~/.jarvis/gists/`

## Key Risks

- **Gmail SPA navigation**: URL changes when navigating threads. User must click "Refresh" in BrowserTool after navigating to a new thread.
- **Obfuscated classes**: We use ZERO class-name selectors. All selectors are ARIA roles, data attributes, or standard HTML attributes.
- **Thread size**: 50K char JS-side guard prevents oversized output. Most threads are well under this.
- **Non-thread views**: Guard returns clear error if user isn't viewing a specific thread.
