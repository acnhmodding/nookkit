# Code Style

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

## Creating new hooks
To create a new hook, the `nookkit_sym::hook!` function should be used. If it's a symbol that does not exist yet, the symbol should be exported by the `ExportSymbols.java` script in Ghidra to the [offsets](offsets) folder.

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
Functions that can error should always return either a `Result<>` or `Option<>` rather than unwrapping or swallowing the error by printing the output.
