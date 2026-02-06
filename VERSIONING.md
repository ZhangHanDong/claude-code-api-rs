# Versioning Strategy

Nexus follows [Semantic Versioning 2.0.0](https://semver.org/).

## Version Format

```
v{MAJOR}.{MINOR}.{PATCH}[-{PRERELEASE}][+{BUILD}]
```

### Examples

| Version | Meaning |
|---------|---------|
| `v0.5.0` | Current stable release |
| `v0.5.1` | Patch release with bug fixes |
| `v0.6.0` | Minor release with new features |
| `v1.0.0` | Major release (breaking changes) |
| `v0.6.0-alpha.1` | Alpha pre-release |
| `v0.6.0-beta.1` | Beta pre-release |
| `v0.6.0-rc.1` | Release candidate |

## Version Rules

### MAJOR (X.0.0)
Increment when making **incompatible API changes**:
- Removing public API
- Changing function signatures
- Changing behavior in breaking ways
- Removing features

### MINOR (0.X.0)
Increment when adding **backward-compatible functionality**:
- New features
- New optional parameters
- Deprecating (not removing) features
- Adding new modules

### PATCH (0.0.X)
Increment for **backward-compatible bug fixes**:
- Bug fixes
- Performance improvements
- Documentation updates
- Internal refactoring (no API changes)

## Pre-release Versions

### Alpha (`-alpha.N`)
Early testing, unstable API:
```
v0.6.0-alpha.1
v0.6.0-alpha.2
```

### Beta (`-beta.N`)
Feature-complete, bug fixing phase:
```
v0.6.0-beta.1
v0.6.0-beta.2
```

### Release Candidate (`-rc.N`)
Final testing before stable:
```
v0.6.0-rc.1
v0.6.0-rc.2
```

## Branch Strategy

```
main                 ← Stable releases only
 ├── v0.5           ← Version 0.5.x maintenance
 │    ├── v0.5.0    (tag)
 │    ├── v0.5.1    (tag)
 │    └── v0.5.2    (tag)
 ├── v0.6           ← Version 0.6.x development
 │    ├── v0.6.0-alpha.1 (tag)
 │    ├── v0.6.0    (tag)
 │    └── v0.6.1    (tag)
 └── feature/*      ← Feature branches
```

## Package Versions

All packages in the workspace share the same version:

| Package | Description |
|---------|-------------|
| `nexus-claude` | Core SDK |
| `claude-code-api` | API server |

Versions are synchronized via `version.workspace = true` in Cargo.toml.

## Changelog

The changelog is automatically generated using [git-cliff](https://git-cliff.org/) based on conventional commits.

### Commit Types

| Type | Description | Changelog Section |
|------|-------------|-------------------|
| `feat` | New feature | Features |
| `fix` | Bug fix | Bug Fixes |
| `perf` | Performance improvement | Performance |
| `refactor` | Code refactoring | Refactor |
| `docs` | Documentation | Documentation |
| `test` | Tests | Testing |
| `chore` | Maintenance | Miscellaneous |

### Breaking Changes

Mark breaking changes with `!` or `BREAKING CHANGE:` footer:

```
feat!: remove deprecated API

BREAKING CHANGE: The `old_method()` has been removed.
Use `new_method()` instead.
```

## Rust MSRV Policy

- Minimum Supported Rust Version (MSRV): **1.88** (required for edition 2024)
- MSRV bumps are considered **minor** version changes
- MSRV is tested in CI on every PR

## Deprecation Policy

1. Features are deprecated in a **minor** release
2. Deprecated features are removed in the next **major** release
3. Deprecation warnings include migration guidance
4. Minimum deprecation period: 2 minor releases

Example:
```
v0.6.0 - `old_api()` deprecated, `new_api()` added
v0.7.0 - `old_api()` still works but warns
v1.0.0 - `old_api()` removed
```

## Release Cadence

- **Patch releases**: As needed for bug fixes
- **Minor releases**: Every 4-8 weeks
- **Major releases**: When significant breaking changes accumulate

## Compatibility

### With Claude Code CLI

| Nexus Version | CLI Version |
|---------------|-------------|
| 0.5.x | 1.x |
| 0.4.x | 1.x |

### With Rust

| Nexus Version | Rust Version |
|---------------|--------------|
| 0.5.x | 1.88+ |
| 0.4.x | 1.75+ |
