# Semantic Fingerprinting

Semantic fingerprinting is rx's approach to avoiding unnecessary rebuilds. Instead of hashing raw file contents, rx parses Rust source files and extracts only the public API surface. Changes that do not affect the public API -- like editing comments, reformatting code, or modifying private function bodies -- do not trigger downstream rebuilds.

## How it works

1. **Parse source files** -- rx uses the `syn` crate to parse each Rust source file into an AST
2. **Extract public API** -- only `pub` items are extracted: functions, structs, enums, traits, type aliases, and impl blocks
3. **Generate stable representation** -- the extracted items are serialized into a stable text format (signatures only, no bodies)
4. **Hash the API surface** -- the text is hashed with xxHash (xxh3-128)

If the hash matches the previous build, downstream dependents are not rebuilt.

## What is included in the fingerprint

- `pub fn` signatures (name, parameters, return type, generics, where clauses)
- `pub struct` definitions (name, fields, generics)
- `pub enum` definitions (name, variants, fields, generics)
- `pub trait` definitions (name, methods, associated types, generics)
- `pub type` aliases
- `pub impl` blocks (method signatures)

## What is NOT included

- Comments and doc comments
- Code formatting and whitespace
- Private functions and their bodies
- Public function bodies (only the signature matters)
- `#[cfg]` attributes on items
- Internal module structure (only the public surface)

## Example

Consider this change:

```rust
// Before
pub fn process(input: &str) -> Result<Output> {
    let parsed = parse(input)?;
    transform(parsed)
}

// After -- body changed, signature unchanged
pub fn process(input: &str) -> Result<Output> {
    let parsed = parse(input)?;
    let validated = validate(parsed)?;
    transform(validated)
}
```

The semantic fingerprint does NOT change because the function signature is identical. Downstream crates that depend on this function are not rebuilt.

Now consider this change:

```rust
// Before
pub fn process(input: &str) -> Result<Output> { ... }

// After -- signature changed (new parameter)
pub fn process(input: &str, opts: Options) -> Result<Output> { ... }
```

The semantic fingerprint DOES change, and downstream crates are rebuilt.

## Fallback behavior

Semantic fingerprinting gracefully degrades:

- **Parse failure** -- if `syn` cannot parse a file (e.g., macro-heavy code), rx falls back to hashing the full file contents
- **No public items** -- if a file has no `pub` items (e.g., a binary crate's `main.rs`), the full content is hashed
- **Non-Rust files** -- files like `build.rs` are always content-hashed

## Workspace impact

In a workspace with packages A, B, and C where B depends on A:

- Editing A's private code: only A is rebuilt
- Editing A's public API: A and B are rebuilt
- Editing C (independent of A and B): only C is rebuilt

This can dramatically reduce build times in large workspaces where most changes are to implementation details rather than public interfaces.
