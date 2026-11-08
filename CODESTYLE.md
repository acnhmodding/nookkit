# Code Style

## Formatting

Run `cargo fmt` before committing and resolve `cargo clippy` warnings. The lints in `[workspace.lints]` enforce parts of this document, every crate opts in with `[lints] workspace = true`.

When a lint has to be silenced, use `#[expect(lint, reason = "...")]` rather than `#[allow]`, so the suppression is explained and goes away once it is no longer needed.

## Readability

Code should be easily skimmable and not clustered into dense blocks. [check_spacing.py](tools/check_spacing.py) checks the blank lines below in CI, run it on your files to add the missing ones.

- Separate distinct steps of a function (reading, computing, writing) with a blank line.
- Put a blank line before and after loops, `if` / `match` blocks and any other multiline statement, unless it is the first or last thing in its scope. A `let ... else { ... };` is multiline too, so consecutive ones are separated by a blank line.
- Put a blank line before the final return expression.
- Consecutive one-line statements that belong together (e.g. a run of `let` reads) can stay grouped.

### Bad

```rs
let name = reader.read_string()?;
let count = reader.read_u8()?;
let mut entries = Vec::with_capacity(count as usize);
for _ in 0..count {
    entries.push(Entry::parse(reader)?);
}
if entries.is_empty() {
    return Err(Error::Empty);
}
Ok(Self { name, entries })
```

### Good

```rs
let name = reader.read_string()?;
let count = reader.read_u8()?;

let mut entries = Vec::with_capacity(count as usize);
for _ in 0..count {
    entries.push(Entry::parse(reader)?);
}

if entries.is_empty() {
    return Err(Error::Empty);
}

Ok(Self { name, entries })
```

### Combinator chains

Chaining `Option` / `Result` combinators (`and_then`, `map`, `or_else`, `filter`, `map_or`, ...) is shorter, but the reader has to unwind every closure in their head to follow it. Prefer `?`, `let ... else`, `if let` or `match`, which name each step and keep the control flow visible.

A single short combinator is fine (`.ok_or(Error::OutOfBounds)?`, `.map(|e| e.name)`), as are plain iterator pipelines like `.iter().filter(..).collect()`. Rewrite a chain once it nests closures, mixes `Option` and `Result`, or needs more than one closure to understand.

### Bad

```rs
let pockets = ui
    .find_page::<pages::PocketMenu>()?
    .and_then(|page| page.seq().ok().flatten())
    .filter(|seq| seq.is_idle().unwrap_or(false));
```

### Good

```rs
let Some(page) = ui.find_page::<pages::PocketMenu>()? else {
    return Ok(());
};
let Some(pockets) = page.seq()? else {
    return Ok(());
};

if !pockets.is_idle()? {
    return Ok(());
}
```

## Comments

Only write a comment to explain a quirk or necessary prerequisite knowledge, it shouldn't simply restate what the code already says.

Comments inside a function body are almost never needed. Clear names and steps separated by blank lines should carry the meaning. Keep one only for something the code cannot express, such as a game bug workaround, an ordering the game depends on or a race.

Don't extract a helper just to replace a comment, a helper used in one place usually muddies the code more than it helps.

Comments are terse and have no trailing period.

### Bad

```rs
// loop over the entries
for entry in &entries {
    // ...
}

// read the header, then the entries
let count = reader.read_u8()?;
let n = reader.read_u32()?;
```

### Good

```rs
// Workaround for MiniGame_PictQuiz_00_MosaicNormal.bflan
if matches!(anim_type, AnimType::User) {
    // ...
}

let count = reader.read_u8()?;
let entry_offset = reader.read_u32()?;
```

### Documentation comments

Until a 1.0 release, keep documentation comments minimal, only on higher level items used across crates. No `//!` module docs for now.

Describe what the item is or does, not how it works. Only mention inner workings if they are necessary to understand it. Like comments, they have no trailing period.

```rs
/// Borrows memory from one of the game's existing heaps
```

On structs and enums, a documented field or variant is separated from the previous one by a blank line (the first needs none):

```rs
pub struct UserData {
    /// The user-defined data stored in this entry
    pub content: UserDataContent,

    /// The name of this user data entry
    pub o_name: String,
}
```

## Pointer offsets

Using offsets in a pointer to access game memory should be avoided in the case that the game updates, new structures should be created in [nookkit-types](crates/nookkit-types).

This doesn't apply to parsers of file formats (e.g. BFRES in nookkit-rstb), which read offsets from the format's own layout. Give an offset a named const when it is used more than once.

### Bad

```rs
// MANAGER_LOADED is the offset to the count of loaded tables
((manager + MANAGER_LOADED) as *const u32).read();
```

### Good

```rs
manager.table_count;
```

## Game types

Structures that mirror game memory are `#[repr(C)]`. When a class derives from another, store the parent as a `base` field and use `nookkit_types::derive_base!`.

## Creating new hooks

To create a new hook, the `nookkit_sym::hook!` function should be used. If it's a symbol that does not exist yet, the symbol should be exported by the [ExportSymbols.java](tools/ghidra/ExportSymbols.java) script in Ghidra to the [offsets](offsets) folder. Add the `tools/ghidra` folder to Ghidra's script directories, then run it headless or from the Script Manager with the absolute paths `<repo>/offsets/<version>.toml <version> <repo>/crates/nookkit-core/symbols.txt <repo>/crates/nookkit-types/symbols.txt` as its arguments.

Files in [offsets](offsets) are generated, do not edit them by hand.

SDK functions that skyline already exposes (`skyline::nn::*`) aren't in the offsets and are hooked with `#[skyline::hook(replace = ...)]` instead.

### Example usage

```rs
nookkit_sym::hook! {
    pub find_file = sym::Lp::UI::MessageArchive::findFile;
    fn(archive: *mut c_void, name: *const SafeString<'static>) -> *mut c_void {
        // ...
    }
}
```

## Errors

Functions that can error should always return either a `Result<>` or `Option<>` rather than unwrapping or swallowing the error.

- Error types are defined with `thiserror` and composed with `#[from]`.
- Keep errors at a good middle ground, not one variant per failure site. Group related failures under one error type, and only give a subsystem its own sub error when it has distinct categories, e.g. zstd and yaz0 compression each have their own error that the parent error wraps.
- Include context in an error, such as the offset, path, or expected versus found value.
- `unwrap()` and `expect()` are only acceptable where failure is impossible because the invariant was just checked. Mods run in the game process with `panic = "abort"`, so a stray panic crashes the whole game.
- Propagate errors with `?`. Only print an error where propagating it would obscure it, and at the top level where it is finally handled.
- Error messages are lowercase and have no trailing period, e.g. `invalid magic, found 0x{found:08X}`.
- Don't discard a `Result` with `let _ =` unless it can't fail, like `write!` into a `String`.

Logging uses `println!` with the crate's prefix, e.g. `[NookKit]`.

### Bad

```rs
// Too granular, one variant per failure site
#[derive(Error, Debug)]
pub enum RstbError {
    #[error("zstd magic was invalid")]
    ZstdInvalidMagic,

    #[error("zstd frame header was truncated")]
    ZstdTruncatedHeader,

    #[error("zstd block was corrupt")]
    ZstdCorruptBlock,

    #[error("yaz0 magic was invalid")]
    Yaz0InvalidMagic,

    #[error("yaz0 data was truncated")]
    Yaz0Truncated,
}
```

### Good

```rs
#[derive(Error, Debug)]
pub enum RstbError {
    #[error("zstd error: {0}")]
    Zstd(#[from] ZstdError),

    #[error("yaz0 error: {0}")]
    Yaz0(#[from] Yaz0Error),

    #[error("unexpected end of file at offset 0x{offset:X}")]
    UnexpectedEof { offset: usize },
}

#[derive(Error, Debug)]
pub enum ZstdError {
    #[error("invalid frame header")]
    InvalidHeader,

    #[error("corrupt block at offset 0x{offset:X}")]
    CorruptBlock { offset: usize },
}

#[derive(Error, Debug)]
pub enum Yaz0Error {
    #[error("invalid magic, found 0x{found:08X}")]
    InvalidMagic { found: u32 },

    #[error("data was truncated")]
    Truncated,
}
```

## Crates

Avoid cyclic crate dependencies unless they are necessary.

Don't create a new crate for a handful of utilities, keep them in the crate that uses them.

If a new crate is absolutely necessary, create it in its own PR, separate from the changes that required it.

## Unsafe code

Keep `unsafe` in the low level crates (`nookkit-types`, `nookkit-sym`, `nookkit-sys` and `nookkit-core`). The `nookkit` crate may use `unsafe` internally, but it exposes only safe APIs. The exception is `original` in hooks made with `nookkit::hook!`, which calls the game's function directly.

## Dependencies

Don't add a dependency without a good reason. Declare it in `[workspace.dependencies]` with `default-features = false` and only enable the features needed.

## Dead code

No commented-out code or unused items.

## Tests

Only write tests for logic that doesn't interact with the game and is complex enough to need them.
