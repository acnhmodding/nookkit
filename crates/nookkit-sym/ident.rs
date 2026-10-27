// Rust identifiers for generated names, shared by the nookkit-sym and nookkit-layouts build scripts

pub const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

// Spelled out so operators that differ only in symbols, like == and !=, get different names
const OPERATORS: &[(&str, &str)] = &[
    ("()", "call"),
    ("[]", "index"),
    ("<<=", "shl_assign"),
    (">>=", "shr_assign"),
    ("<<", "shl"),
    (">>", "shr"),
    ("==", "eq"),
    ("!=", "ne"),
    ("<=", "le"),
    (">=", "ge"),
    ("&&", "and"),
    ("||", "or"),
    ("++", "inc"),
    ("--", "dec"),
    ("->", "arrow"),
    ("+=", "add_assign"),
    ("-=", "sub_assign"),
    ("*=", "mul_assign"),
    ("/=", "div_assign"),
    ("%=", "rem_assign"),
    ("&=", "and_assign"),
    ("|=", "or_assign"),
    ("^=", "xor_assign"),
    ("=", "assign"),
    ("<", "lt"),
    (">", "gt"),
    ("+", "add"),
    ("-", "sub"),
    ("*", "mul"),
    ("/", "div"),
    ("%", "rem"),
    ("&", "bitand"),
    ("|", "bitor"),
    ("^", "xor"),
    ("~", "bitnot"),
    ("!", "not"),
];

/// Any name as a raw-safe Rust identifier
pub fn ident(segment: &str) -> String {
    let segment = match segment.strip_prefix("operator") {
        Some(op)
            if !op.is_empty() && !op.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') =>
        {
            let op = op.trim();
            let word = OPERATORS
                .iter()
                .find(|(symbols, _)| *symbols == op)
                .map_or(op, |&(_, word)| word);

            format!("operator_{word}")
        }
        _ => segment.to_owned(),
    };

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
        "" | "_" => "__".to_owned(),
        "self" | "Self" | "super" | "crate" => out + "_",
        kw if KEYWORDS.contains(&kw) => format!("r#{kw}"),
        _ => out,
    }
}
