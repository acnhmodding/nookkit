// One typed constant per symbol, so a name missing from the symbol file fails to compile

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::{env, fs, path::PathBuf};

#[allow(dead_code)]
#[path = "src/parse.rs"]
mod parse;

use parse::Kind;

#[path = "ident.rs"]
mod ident;

use ident::ident;

const SYMBOLS: &str = "../../offsets/3.0.3.toml";
const SIGNATURES: &str = "../../offsets/signatures.toml";

#[derive(Default)]
struct Module {
    modules: BTreeMap<String, Module>,
    items: BTreeMap<String, String>,
}

/// Keeps `<...>` template arguments in one segment
fn segments(name: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    let bytes = name.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // operator<, operator<< and friends name a function, they don't open a template
        if name[i..].starts_with("operator") {
            i += "operator".len();
            while i < bytes.len() && b"<>=".contains(&bytes[i]) {
                i += 1;
            }
            continue;
        }

        match bytes[i] {
            b'<' => depth += 1,
            b'>' => depth = depth.saturating_sub(1),
            b':' if depth == 0 && bytes.get(i + 1) == Some(&b':') => {
                out.push(&name[start..i]);
                start = i + 2;
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    out.push(&name[start..]);
    out
}

fn abi_type(token: &str) -> Option<&'static str> {
    Some(match token {
        "ptr" => "crate::abi::Ptr",
        "i8" => "crate::abi::I8",
        "i16" => "crate::abi::I16",
        "i32" => "crate::abi::I32",
        "i64" => "crate::abi::I64",
        "f32" => "crate::abi::F32",
        "f64" => "crate::abi::F64",
        "void" => "crate::abi::Void",
        _ => return None,
    })
}

/// `"ptr, i32 -> i8"` to `crate::Sig<(Ptr, I32,), I8>`
fn signature(text: &str) -> Result<String, String> {
    let (args, ret) = text.split_once("->").ok_or("missing `->`")?;
    let args: Vec<_> = args
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(|a| abi_type(a).ok_or(format!("unknown ABI type `{a}`")))
        .collect::<Result<_, _>>()?;
    let ret = abi_type(ret.trim()).ok_or(format!("unknown ABI type `{}`", ret.trim()))?;

    let args: String = args.iter().map(|a| format!("{a}, ")).collect();
    Ok(format!("crate::Sig<({args}), {ret}>"))
}

fn signatures(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();

    for (index, raw) in text.lines().enumerate() {
        let line = parse::strip_comment(raw).trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }

        let (name, sig) = line
            .split_once('=')
            .and_then(|(k, v)| Some((k.trim().strip_prefix('"')?.strip_suffix('"')?, v.trim())))
            .unwrap_or_else(|| {
                panic!(
                    "{SIGNATURES}:{}: expected \"name\" = \"signature\"",
                    index + 1
                )
            });
        let sig = sig.trim_matches('"');
        let ty = signature(sig).unwrap_or_else(|e| panic!("{SIGNATURES}:{}: {e}", index + 1));

        if out.insert(name.to_owned(), ty).is_some() {
            panic!(
                "{SIGNATURES}:{}: duplicate signature for \"{name}\"",
                index + 1
            );
        }
    }

    out
}

fn emit(module: &Module, out: &mut String, depth: usize) {
    let pad = "    ".repeat(depth);

    for (name, item) in &module.items {
        writeln!(out, "{pad}pub const {name}: {item};").unwrap();
    }
    for (name, child) in &module.modules {
        writeln!(out, "{pad}pub mod {name} {{").unwrap();
        emit(child, out, depth + 1);
        writeln!(out, "{pad}}}").unwrap();
    }
}

fn main() {
    println!("cargo::rerun-if-changed={SYMBOLS}");
    println!("cargo::rerun-if-changed={SIGNATURES}");

    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let file = parse::parse(&fs::read_to_string(dir.join(SYMBOLS)).expect("read the symbol file"))
        .unwrap_or_else(|e| panic!("{SYMBOLS}: {e}"));
    // Without it every hook's signature check would silently pass
    let sigs = signatures(
        &fs::read_to_string(dir.join(SIGNATURES))
            .unwrap_or_else(|e| panic!("read {SIGNATURES}: {e}")),
    );

    for name in sigs.keys() {
        if file.get(Kind::Fn, name).is_none() {
            println!("cargo::warning=signature for \"{name}\", which isn't in [fn] of {SYMBOLS}");
        }
    }

    let mut root = Module::default();
    let mut seen = HashMap::new();
    let mut entries: Vec<_> = file.iter().collect();
    entries.sort_by_key(|&(_, name, _)| name);

    for (kind, name, _) in entries {
        let parts: Vec<String> = segments(name).into_iter().map(ident).collect();
        let (last, path) = parts.split_last().expect("symbol names are never empty");

        if let Some(other) = seen.insert(parts.join("::"), name) {
            panic!("\"{name}\" and \"{other}\" map to the same constant");
        }

        let module = path
            .iter()
            .fold(&mut root, |m, p| m.modules.entry(p.clone()).or_default());
        let item = match kind {
            Kind::Fn => match sigs.get(name) {
                Some(sig) => format!("crate::Fn<{sig}> = crate::Fn::new({name:?})"),
                None => format!("crate::Fn = crate::Fn::new({name:?})"),
            },
            Kind::Vtable => format!("crate::Vtable = crate::Vtable::new({name:?})"),
            Kind::Data => format!("crate::Data = crate::Data::new({name:?})"),
        };
        module.items.insert(last.clone(), item);
    }

    let mut out = String::new();
    emit(&root, &mut out, 0);

    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("sym.rs");
    fs::write(path, out).expect("write sym.rs");
}
