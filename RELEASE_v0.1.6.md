# Release v0.1.6 Summary

## Date: 2025-01-23

### What's New

This release brings the Rust SDK to full feature parity with the Python SDK v0.0.19.

### New Features

1. **Settings File Support**
   - Added `settings` field to `ClaudeCodeOptions`
   - Allows specifying custom Claude Code settings files via `--settings` CLI parameter
   - Example: `.settings("path/to/settings.json")`

2. **Multiple Working Directories**
   - Added `add_dirs` field to `ClaudeCodeOptions`
   - Support for adding multiple working directories via `--add-dir` CLI parameter
   - Two methods available:
     - `.add_dir(path)` - Add directories one by one
     - `.add_dirs(vec![paths])` - Add multiple directories at once

### API Changes

#### ClaudeCodeOptions
```rust
pub struct ClaudeCodeOptions {
    // ... existing fields ...
    pub settings: Option<String>,        // NEW
    pub add_dirs: Vec<PathBuf>,         // NEW
}
```

#### ClaudeCodeOptionsBuilder
```rust
// New builder methods
.settings(path)           // Set settings file path
.add_dir(path)           // Add a single directory
.add_dirs(vec![paths])   // Add multiple directories
```

### Examples

#### Using Settings File
```rust
let options = ClaudeCodeOptions::builder()
    .settings("/path/to/settings.json")
    .build();
```

#### Adding Multiple Directories
```rust
let options = ClaudeCodeOptions::builder()
    .cwd("/main/project")
    .add_dir("/additional/project1")
    .add_dir("/additional/project2")
    .build();
```

### Test Examples

New examples have been added to demonstrate the features:
- `demo_new_features.rs` - Feature demo without Claude CLI
- `test_settings.rs` - Settings file usage
- `test_settings_safe.rs` - Safe settings handling
- `test_add_dirs.rs` - Multiple directories
- `test_combined_features.rs` - Combined features
- `test_paths.rs` - Path verification utility

### Configuration Files

Example configuration files provided:
- `examples/claude-settings.json` - Basic configuration
- `examples/custom-claude-settings.json` - Advanced configuration with MCP servers

### Testing

All tests pass with the new features:
```bash
cargo test              # Run unit tests
cargo test -- --ignored # Run integration tests (requires Claude CLI)
```

### Notes

- When using settings files, it's recommended to use absolute paths
- The SDK now has complete feature parity with Python SDK v0.0.19
- Integration tests requiring Claude CLI are marked with `#[ignore]`

### Breaking Changes

None. This release is fully backward compatible.

### Dependencies

No new dependencies added.

### Migration Guide

No migration needed. Existing code will continue to work without changes.