# Publishing to crates.io

## Pre-publication Checklist

- [x] README in English, Chinese, and Japanese
- [x] CHANGELOG with version history
- [x] LICENSE file (MIT)
- [x] Updated Cargo.toml with:
  - Version 0.1.5
  - Proper metadata (description, keywords, categories)
  - Repository/homepage URLs (update with your actual GitHub URL)
  - Exclude unnecessary files
- [x] Clean up unused test files
- [x] Documentation comments for public APIs
- [x] Examples in the examples/ directory

## Steps to Publish

1. **Update GitHub URLs**
   - Replace `your-username` in Cargo.toml with your actual GitHub username
   - Create the repository on GitHub if not already done

2. **Final checks**
   ```bash
   cargo check
   cargo test
   cargo doc --open  # Review documentation
   cargo package --list  # Review what will be included
   ```

3. **Dry run**
   ```bash
   cargo publish --dry-run
   ```

4. **Publish**
   ```bash
   cargo publish
   ```

## Post-publication

1. Create a GitHub release with tag `v0.1.5`
2. Update any downstream projects
3. Announce on relevant channels

## Important Notes

- The crate name `claude-code-sdk` must be available on crates.io
- You need to be logged in with `cargo login`
- Once published, versions cannot be deleted (only yanked)
- Make sure all tests pass before publishing

## Version 0.1.5 Highlights

- Fixed critical interactive mode deadlock issue
- Added `InteractiveClient` with full Python SDK parity
- New methods: `send_message()`, `receive_response()`, `interrupt()`
- Broadcast channel support for multiple message receivers
- Comprehensive documentation in three languages