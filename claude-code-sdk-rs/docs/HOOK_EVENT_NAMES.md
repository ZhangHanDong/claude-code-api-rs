# Hook Event Names Reference

## ⚠️ Important: Event Names Must Use PascalCase

When registering hooks with the Claude Code SDK, **event names must be in PascalCase** to match the CLI's expectations. Using snake_case or other formats will cause hooks to never trigger.

## ✅ Correct Event Names

| Hook Event | Correct Format | ❌ Wrong Format |
|------------|----------------|-----------------|
| Pre Tool Use | `"PreToolUse"` | `"pre_tool_use"`, `"pre-tool-use"` |
| Post Tool Use | `"PostToolUse"` | `"post_tool_use"`, `"post-tool-use"` |
| User Prompt Submit | `"UserPromptSubmit"` | `"user_prompt_submit"`, `"user-prompt-submit"` |
| Stop | `"Stop"` | `"stop"` |
| Subagent Stop | `"SubagentStop"` | `"subagent_stop"`, `"subagent-stop"` |
| Pre Compact | `"PreCompact"` | `"pre_compact"`, `"pre-compact"` |

## 📝 Example Usage

### Correct ✅

```rust
use nexus_claude::{ClaudeCodeOptions, HookMatcher};
use std::collections::HashMap;

let mut hooks = HashMap::new();

// ✅ Correct: PascalCase
hooks.insert(
    "PreToolUse".to_string(),
    vec![HookMatcher {
        matcher: Some(serde_json::json!("*")),
        hooks: vec![Arc::new(MyHook)],
    }],
);

hooks.insert(
    "PostToolUse".to_string(),
    vec![HookMatcher {
        matcher: Some(serde_json::json!("*")),
        hooks: vec![Arc::new(MyHook)],
    }],
);
```

### Incorrect ❌

```rust
// ❌ Wrong: snake_case - hooks will never trigger!
hooks.insert(
    "pre_tool_use".to_string(),  // This will NOT work
    vec![...],
);

// ❌ Wrong: kebab-case - hooks will never trigger!
hooks.insert(
    "post-tool-use".to_string(),  // This will NOT work
    vec![...],
);
```

## 🔍 Why PascalCase?

The Claude Code CLI uses PascalCase for all event names in its internal protocol. When you register a hook:

1. **SDK → CLI**: The SDK sends hook configurations to the CLI with the event names you provide
2. **CLI Processing**: The CLI matches incoming events against registered hook names
3. **Event Triggering**: Only exact matches (case-sensitive) will trigger your hooks

**Example Flow**:

```
1. You register: "PreToolUse"
   ✅ CLI receives: "PreToolUse"
   ✅ CLI event fires: "PreToolUse"
   ✅ Match! → Hook executes

2. You register: "pre_tool_use"
   ❌ CLI receives: "pre_tool_use"
   ❌ CLI event fires: "PreToolUse"
   ❌ No match → Hook never executes
```

## 🧪 Debugging Hook Registration

If your hooks aren't triggering, check:

1. **Event Name Format**: Ensure PascalCase
   ```rust
   // Check your registration
   println!("Registered hooks: {:?}", hooks.keys());
   // Should see: ["PreToolUse", "PostToolUse"]
   // NOT: ["pre_tool_use", "post_tool_use"]
   ```

2. **Enable Debug Output**:
   ```rust
   options.debug_stderr = Some(Arc::new(Mutex::new(std::io::stderr())));
   ```

3. **Check Hook Initialization**:
   Look for CLI logs showing registered hooks:
   ```
   Hooks registered: {
     "PreToolUse": [...],
     "PostToolUse": [...]
   }
   ```

## 📚 See Also

- [Hook Examples](../examples/hooks_typed.rs) - Complete working example with strongly-typed hooks
- [Control Protocol Demo](../examples/control_protocol_demo.rs) - Advanced usage with permissions and hooks
- [Implementation Summary](../../HOOK_TYPES_IMPLEMENTATION_SUMMARY.md) - Technical details of the hook system

## 🔗 Related to Type Safety

The strongly-typed `HookInput` enum also uses PascalCase in its variant names, which aligns with the event names:

```rust
pub enum HookInput {
    #[serde(rename = "PreToolUse")]    // ← Same as event name
    PreToolUse(PreToolUseHookInput),

    #[serde(rename = "PostToolUse")]   // ← Same as event name
    PostToolUse(PostToolUseHookInput),

    // ...
}
```

This consistency helps ensure correctness across the entire hook system.
