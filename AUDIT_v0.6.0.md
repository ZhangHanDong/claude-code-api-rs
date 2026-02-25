# rust-claude-code-api v0.6.0 Quality Audit Report

**Date:** 2026-02-11
**Auditors:** Claude Code (team lead) + Codex (security) + Gemini CLI (API ergonomics)
**Method:** 3-phase diamond DAG, native CC team orchestration

## DAG Execution Summary

```
Phase 1:  T1(CC)●  T2(Codex)●  T3(Gemini)●     ← 3/3 delivered
              ↘  ↙       ↘  ↙
Phase 2:   T4(Codex)●   T5(Gemini)✗             ← 1/2 (Gemini 429)
                 ↘      ↙
Phase 3:       T6(CC)●                           ← team lead synthesis
```

| Agent | Backend | Task | Status | Tokens |
|-------|---------|------|--------|--------|
| cc-arch | Claude Code | Architecture deep-dive | Done | — |
| codex-security | Codex | Security review | Done | 110K |
| gemini-api | Gemini CLI | API ergonomics | Done | — |
| codex-fixes | Codex | Code improvements | Done | 48K |
| gemini-refine | Gemini CLI | API refinements | 429 rate limit | 0 |
| team-lead | Claude Code | Synthesis | Done | — |

---

## Phase 1: Gemini API Ergonomics Review

All 6 categories rated **GOOD**:

1. **Builder patterns: GOOD** — `ClaudeCodeOptionsBuilder` is comprehensive with fluent chaining and `add_*` methods
2. **Error types: GOOD** — `SdkError` uses `thiserror`, has `is_recoverable()` helper and `From` impls
3. **Type re-exports: GOOD** — Clean `prelude` module, extensive `pub use` in lib.rs
4. **Naming: GOOD** — Follows Rust conventions, proper `serde(rename_all = "camelCase")` for JSON
5. **Documentation: GOOD** — Comprehensive doc comments with examples, `#![warn(missing_docs)]` enabled
6. **Convenience methods: GOOD** — `receive_response`, `ToolsConfig::none()`, `rewind_files`

---

## Phase 1: Codex Security Findings (8 issues)

### P0 — Must Fix Before Release

#### 1. CRITICAL: Remote script execution without integrity verification
- **Files:** `claude-code-sdk-rs/src/cli_download.rs:251-254`, `:349-353`
- **Issue:** Downloads installer scripts from network and executes directly (`bash -c <downloaded_script>` and PowerShell `iex (iwr ...)`).
- **Impact:** Full RCE if upstream script/CDN is compromised or traffic is intercepted.
- **Fix:** Add `sha2` checksum verification before execution; write to temp file; execute file not string.

#### 2. HIGH: PowerShell command injection on Windows installer path
- **Files:** `claude-code-sdk-rs/src/cli_download.rs:349-353`
- **Issue:** `parent_dir` is interpolated into a PowerShell `-Command` string without escaping.
- **Impact:** If path contains `'`/special chars, command string can be broken/injected.
- **Fix:** Use `-File` with structured args; eliminate string interpolation entirely.

#### 3. HIGH: SSRF and unvalidated file-path passthrough in image handling
- **Files:** `claude-code-api/src/api/chat.rs:297-301`, `:304-334`
- **Issue:** Arbitrary `http(s)` URLs are fetched server-side; non-URL values are passed through as file paths.
- **Impact:** SSRF to internal services, large-response DoS, and potential local-file access.
- **Fix:** Parse URL with `reqwest::Url`, enforce HTTPS-only, add SSRF guard (`validate_not_private_or_loopback`), enforce content-type + size limits; reject arbitrary local paths.

#### 4. HIGH: Sensitive internal errors returned to API clients
- **Files:** `claude-code-api/src/models/error.rs:84`, variants `:11-52`
- **Issue:** API responses include `self.to_string()` for all errors, leaking DB/config/IO/process details.
- **Impact:** Information disclosure useful for reconnaissance and credential/path leakage.
- **Fix:** Return generic client-safe messages for 5xx; log full details server-side only; map internal errors to opaque codes.

### P1 — Should Fix

#### 5. MEDIUM: IPC line parsing is unbounded (memory DoS risk)
- **Files:** `claude-code-sdk-rs/src/transport/subprocess.rs:710-713`, `query.rs:335-338`, `claude-code-api/src/core/claude_manager.rs:89-92`, `interactive_session.rs:318-321`, `session_process.rs:102-110`
- **Issue:** `BufRead::lines` / `read_line` consume unbounded line lengths from subprocess output.
- **Fix:** Use `tokio_util::codec::LinesCodec::new_with_max_length(64 * 1024)`.

#### 6. MEDIUM: Async race/locking correctness issues around session state
- **Files:** `claude-code-api/src/core/interactive_session.rs:82-85`, `:201-204`, `claude_manager.rs:252-277`
- **Issue:** TOCTOU (`contains_key` then create), and `stdin` is temporarily removed then reinserted across await points.
- **Fix:** Double-check under write lock, or use `entry()` API for atomic insert.

#### 7. MEDIUM: Unsafe environment mutation in multi-threaded contexts
- **Files:** `claude-code-sdk-rs/src/client.rs:122-124,165-167`, `query.rs:123-125`, `interactive.rs:26-28`, `optimized_client.rs:110-112`, `client_working.rs:36-38`
- **Issue:** `unsafe { std::env::set_var(...) }` is used repeatedly at runtime. In Rust 2024 this is UB.
- **Fix:** Remove all `set_var` calls; pass `CLAUDE_CODE_ENTRYPOINT` via `cmd.env()` per subprocess.

#### 8. LOW: Request-ID middleware can panic on malformed header
- **Files:** `claude-code-api/src/middleware/request_id.rs:24`, `:31`
- **Issue:** `request_id.parse().unwrap()` on user-controlled header.
- **Fix:** Replace `unwrap()` with fallible handling; regenerate UUID on parse failure.

---

## Phase 2: Codex Code Fixes (before/after)

### Fix 1: Unverified installer execution (CRITICAL)

**Before** (`cli_download.rs:251-254`):
```rust
let output = Command::new("bash")
    .arg("-c")
    .arg(&script_content)
    .env("CLAUDE_INSTALL_DIR", parent_dir)
    .output()
    .await?;
```

**After:**
```rust
use sha2::{Digest, Sha256};
const INSTALL_SH_SHA256: &str = "pinned_hex_digest_here";

let actual = hex::encode(Sha256::digest(script_content.as_bytes()));
if actual != INSTALL_SH_SHA256 {
    return Err(SdkError::SecurityError("install.sh checksum mismatch".into()));
}

let script_path = temp_dir.join("install.sh");
tokio::fs::write(&script_path, &script_content).await?;
let output = Command::new("bash")
    .arg(&script_path)
    .env("CLAUDE_INSTALL_DIR", parent_dir)
    .output()
    .await?;
```

### Fix 2: PowerShell command injection (HIGH)

**Before** (`cli_download.rs:349-353`):
```rust
"-Command",
&format!(
  "$env:CLAUDE_INSTALL_DIR='{}'; iex (iwr -useb {})",
  parent_dir.display(), install_script_url
),
```

**After:**
```rust
let script = download_and_verify_ps1(install_script_url).await?;
let script_path = temp_dir.join("install.ps1");
tokio::fs::write(&script_path, script).await?;

let output = Command::new("powershell")
    .args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File"])
    .arg(&script_path)
    .env("CLAUDE_INSTALL_DIR", parent_dir)
    .output()
    .await?;
```

### Fix 3: SSRF + file path passthrough (HIGH)

**Before** (`api/chat.rs:297-334`):
```rust
} else if url.starts_with("http://") || url.starts_with("https://") {
    download_image(url).await
} else {
    Ok(url.to_string())
}
```

**After:**
```rust
let parsed = reqwest::Url::parse(url)
    .map_err(|_| ApiError::BadRequest("Invalid image URL".into()))?;

if parsed.scheme() != "https" {
    return Err(ApiError::BadRequest("Only https image URLs are allowed".into()));
}
validate_not_private_or_loopback(&parsed).await?;
download_image_with_limits(&parsed).await
```

### Fix 4: Internal error leakage (HIGH)

**Before** (`models/error.rs:84`):
```rust
message: self.to_string(),
```

**After:**
```rust
let public_message = match &self {
    ApiError::BadRequest(m) | ApiError::Unauthorized(m) | ApiError::NotFound(m)
    | ApiError::InvalidModel(m) | ApiError::ContextLengthExceeded(m) => m.clone(),
    _ => "Internal server error".to_string(),
};
if matches!(self, ApiError::Database(_) | ApiError::Config(_) | ApiError::Io(_) | ApiError::Json(_)) {
    tracing::error!(error = ?self, "internal api error");
}
message: public_message,
```

### Fix 5: Unbounded line parsing (MEDIUM)

**Before:**
```rust
let mut lines = reader.lines();
while let Ok(Some(line)) = lines.next_line().await { ... }
```

**After:**
```rust
use tokio_util::codec::{FramedRead, LinesCodec};
let mut lines = FramedRead::new(reader, LinesCodec::new_with_max_length(64 * 1024));
while let Some(line) = lines.next().await {
    let line = line.map_err(|_| SdkError::ProtocolError("line too long".into()))?;
    ...
}
```

### Fix 6: TOCTOU session race (MEDIUM)

**Before** (`interactive_session.rs:82-85`):
```rust
let session_exists = self.sessions.read().contains_key(&conversation_id);
if session_exists { ... } else { self.create_session(...).await?; }
```

**After:**
```rust
let existing = { self.sessions.read().get(&conversation_id).cloned() };
if let Some(session) = existing {
    reuse_session(session, message, response_tx).await;
} else {
    let should_create = { !self.sessions.write().contains_key(&conversation_id) };
    if should_create { self.create_session(conversation_id.clone(), model, message, response_tx).await?; }
}
```

### Fix 7: Unsafe `set_var` in async runtime (MEDIUM)

**Before** (6 locations):
```rust
unsafe { std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust"); }
```

**After:**
```rust
// When spawning Claude process:
cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
```

### Fix 8: Request-ID panic DoS (LOW)

**Before** (`request_id.rs:24,31`):
```rust
request_id.parse().unwrap()
```

**After:**
```rust
let hv = axum::http::HeaderValue::from_str(&request_id)
    .unwrap_or_else(|_| axum::http::HeaderValue::from_str(&Uuid::new_v4().to_string()).unwrap());
req.headers_mut().insert(X_REQUEST_ID.clone(), hv.clone());
response.headers_mut().insert(X_REQUEST_ID.clone(), hv);
```

---

## P2 — Nice to Have (API Improvements)

| # | Proposal | Breaking? | Effort |
|---|----------|-----------|--------|
| 9 | **Retry helper** — `query_with_retry(prompt, RetryPolicy)` with exponential backoff | No | M |
| 10 | **Type-state `InteractiveClient`** — `InteractiveClient<Disconnected>` → `InteractiveClient<Connected>` prevents misuse | Yes | L |
| 11 | **Timeout wrapper** — `ClaudeCodeOptionsBuilder::timeout(Duration)` that auto-cancels | No | S |
| 12 | **Connection pool surfacing** — expose `OptimizedClient` pool metrics via public API | No | S |

---

## Recommended Implementation Order

```
Week 1:  #1 (CRITICAL RCE) → #2 (injection) → #4 (error leak) → #8 (panic)
Week 2:  #3 (SSRF) → #7 (set_var UB) → #6 (TOCTOU)
Week 3:  #5 (line bounds) → #9 (retry) → #11 (timeout)
Future:  #10 (type-state) → #12 (pool metrics)
```

---

## Backend Performance Notes

| Backend | Strengths | Weaknesses |
|---------|-----------|------------|
| **Claude Code** (cc-sdk) | Best synthesis, structured output, reliable | Slowest startup (~3s) |
| **Codex** (exec mode) | Deep code analysis, reads files autonomously, 110K token reviews | Needs `--full-auto` flag, slow shutdown |
| **Gemini CLI** (pipe mode) | Fast when it works, good for architecture overviews | Rate limits (429), fragile in pipe mode |
