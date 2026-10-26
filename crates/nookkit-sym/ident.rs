// Rust identifiers for generated names, shared by the nookkit-sym and nookkit-layouts build scripts

pub const KEYWORDS: &[&str] = &[
    "as", "async", "await", "box", "break", "const", "continue", "dyn", "else", "enum", "extern",
    "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match",
    "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static", "struct", "trait",
    "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while",
    "yield",
];

/// Any name as a raw-safe Rust identifier
pub fn ident(segment: &str) -> String {
    let mut out: String = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    while out.ends_with('_') && out.len() > 1 {
        out.pop();
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }

    match out.as_str() {
        "self" | "Self" | "super" | "crate" => out + "_",
        kw if KEYWORDS.contains(&kw) => format!("r#{kw}"),
        _ => out,
    }
}
