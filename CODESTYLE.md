# Code Style

## Formatting

Run `cargo fmt` before committing and resolve `cargo clippy` warnings. Dependencies shared between crates are declared under `[workspace.dependencies]` and referenced with `{ workspace = true }`.

## Readability

Code should be easily skimmable and not clustered into dense blocks.

- Separate distinct steps of a function (reading, computing, writing) with a blank line.
- Put a blank line before and after loops, `if` / `match` blocks and any other multiline statement, unless it is the first or last thing in its scope.
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

## Comments

Only write a comment to explain a quirk or necessary prerequisite knowledge, it shouldn't simply restate what the code already says.

### Bad

```rs
// loop over the entries
for entry in &entries {
    // ...
}
```

### Good

```rs
// Workaround for MiniGame_PictQuiz_00_MosaicNormal.bflan
if matches!(anim_type, AnimType::User) {
    // ...
}
```

### Documentation comments

Until a 1.0 release, keep documentation comments minimal, only on higher level items used across crates.

Describe what the item is or does, not how it works. Only mention inner workings if they are necessary to understand it.

```rs
/// Borrows memory from one of the game's existing heaps.
```

On structs and enums, a documented field or variant is separated from the previous one by a blank line (the first needs none):

```rs
pub struct UserData {
    /// The user-defined data stored in this entry.
    pub content: UserDataContent,

    /// The name of this user data entry.
    pub o_name: String,
}
```

## Pointer offsets

Using offsets in a pointer to access data should be avoided in the case that the game updates, new structures should be created in [nookkit-types](crates/nookkit-types).

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

To create a new hook, the `nookkit_sym::hook!` function should be used. If it's a symbol that does not exist yet, the symbol should be exported by the `ExportSymbols.java` script in Ghidra to the [offsets](offsets) folder.

Files in [offsets](offsets) are generated, do not edit them by hand.

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

Keep `unsafe` in the low level crates (`nookkit-types`, `nookkit-sym`, `nookkit-sys` and `nookkit-core`). The `nookkit` crate should expose only safe APIs.

## Dependencies

Don't add a dependency without a good reason. Declare it in `[workspace.dependencies]` with `default-features = false` and only enable the features needed.

## Dead code

No commented-out code or unused items.

## Tests

Only write tests for logic that doesn't interact with the game and is complex enough to need them.
