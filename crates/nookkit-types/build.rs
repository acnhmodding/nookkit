// Generates game::save's schema structs from the harvested save schema
#![expect(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "build scripts report errors by panicking"
)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::{env, fs, path::PathBuf};

#[path = "../nookkit-sym/ident.rs"]
mod ident;

use ident::ident;

const SAVE: &str = "../../data/save-3.0.3.toml";

#[derive(Default)]
struct Type {
    size: usize,
    align: usize,
    fields: Vec<Field>,
}

struct Field {
    name: String,
    offset: usize,
    kind: String,
    dims: Vec<usize>,
}

fn unquote(s: &str) -> &str {
    let s = s.trim();

    match s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(inner) => inner,
        None => s,
    }
}

fn number(s: &str, at: usize) -> usize {
    let parsed = match s.strip_prefix("0x") {
        Some(hex) => usize::from_str_radix(hex, 16),
        None => s.parse(),
    };

    parsed.unwrap_or_else(|_| panic!("{SAVE}:{at}: bad number `{s}`"))
}

// Only the subset tools/harvest_save.py writes: `["type"]`, `size`, `align`, `field.<name> = "<offset> <type> [<len>[x<len>]]"`
fn schema(text: &str) -> BTreeMap<String, Type> {
    let mut out: BTreeMap<String, Type> = BTreeMap::new();
    let mut current = None;

    for (index, raw) in text.lines().enumerate() {
        let at = index + 1;
        let line = raw.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let name = unquote(name).to_owned();
            out.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }

        let (key, value) = line
            .split_once(" = ")
            .unwrap_or_else(|| panic!("{SAVE}:{at}: expected key = value"));

        let Some(ty) = current.as_ref().and_then(|name| out.get_mut(name)) else {
            panic!("{SAVE}:{at}: key outside a table");
        };

        match key.strip_prefix("field.") {
            Some(name) => {
                let mut parts = unquote(value).split(' ');

                let (Some(offset), Some(kind)) = (parts.next(), parts.next()) else {
                    panic!("{SAVE}:{at}: expected `<offset> <type>`");
                };

                let dims = match parts.next() {
                    Some(dims) => dims.split('x').map(|n| number(n, at)).collect(),
                    None => Vec::new(),
                };

                ty.fields.push(Field {
                    name: name.to_owned(),
                    offset: number(offset, at),
                    kind: kind.to_owned(),
                    dims,
                });
            }
            None if key == "size" => ty.size = number(value, at),
            None if key == "align" => ty.align = number(value, at),
            None => panic!("{SAVE}:{at}: unknown key `{key}`"),
        }
    }

    out
}

// `::Game::SaveLand` to `SaveLand`, `Unknown_5B202DA4` to `Unknown5B202DA4`. A type nested in another
// takes its parent's name, `::Game::SaveWeather::SaveAuroraPattern` to `SaveWeatherAuroraPattern`
fn type_ident(name: &str, schema: &BTreeMap<String, Type>) -> String {
    let Some((parent, leaf)) = name.rsplit_once("::") else {
        return ident(&name.replace('_', ""));
    };

    if !schema.contains_key(parent) {
        return ident(&leaf.replace('_', ""));
    }

    let leaf = match leaf.strip_prefix("Save") {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_uppercase()) => rest,
        _ => leaf,
    };

    ident(&format!("{}{leaf}", type_ident(parent, schema)))
}

// `HasPlayerItemSaveAfter` to `has_player_item_save_after`
fn field_ident(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();

    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1);

        let hump = c.is_ascii_uppercase()
            && prev.is_some_and(|p| {
                p.is_ascii_lowercase()
                    || p.is_ascii_digit()
                    || (p.is_ascii_uppercase() && next.is_some_and(char::is_ascii_lowercase))
            });

        if hump && !out.ends_with('_') {
            out.push('_');
        }

        out.push(c.to_ascii_lowercase());
    }

    ident(&out)
}

// Bools stay u8 so a stray byte in the save can't be an invalid bool
fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "bool" | "u8" => "u8",
        "char16" | "u16" => "u16",
        "u32" => "u32",
        "u64" => "u64",
        "s8" => "i8",
        "s16" => "i16",
        "s32" => "i32",
        "s64" => "i64",
        "f32" => "f32",
        _ => return None,
    })
}

fn leaf(name: &str, rust: &str, ty: &Type, out: &mut String) {
    writeln!(out, "/// `{name}`").unwrap();
    writeln!(out, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]").unwrap();

    let int = match (ty.size == ty.align, ty.size) {
        (true, 1) => Some("u8"),
        (true, 2) => Some("u16"),
        (true, 4) => Some("u32"),
        (true, 8) => Some("u64"),
        _ => None,
    };

    match int {
        Some(int) => {
            writeln!(out, "#[repr(transparent)]").unwrap();
            writeln!(out, "pub struct {rust}(pub {int});").unwrap();
        }
        None => {
            writeln!(out, "#[repr(C, align({}))]", ty.align).unwrap();
            writeln!(out, "pub struct {rust}(pub [u8; {:#x}]);", ty.size).unwrap();
        }
    }
}

fn record(
    name: &str,
    rust: &str,
    ty: &Type,
    schema: &BTreeMap<String, Type>,
    types: &HashMap<&str, String>,
    out: &mut String,
) {
    let mut asserts = String::new();
    let mut seen = HashMap::new();
    let mut end = 0;

    writeln!(out, "/// `{name}`").unwrap();
    writeln!(out, "#[repr(C, align({}))]", ty.align).unwrap();
    writeln!(out, "pub struct {rust} {{").unwrap();

    for field in &ty.fields {
        let field_name = field_ident(&field.name);

        if let Some(other) = seen.insert(field_name.clone(), &field.name) {
            panic!(
                "{name}: fields `{}` and `{other}` map to the same name",
                field.name
            );
        }

        if field.offset < end {
            panic!("{name}: field `{}` overlaps the previous one", field.name);
        }

        if field.offset > end {
            writeln!(out, "    _unk_{end:x}: [u8; {:#x}],", field.offset - end).unwrap();
        }

        let elem = match (primitive(&field.kind), types.get(field.kind.as_str())) {
            (Some(p), _) => p.to_owned(),
            (None, Some(t)) => t.clone(),
            (None, None) => panic!(
                "{name}: field `{}` has unknown type `{}`",
                field.name, field.kind
            ),
        };

        let count: usize = field.dims.iter().product();

        let field_type = match field.dims.as_slice() {
            [] => elem,
            [_] => format!("[{elem}; {count}]"),
            dims => {
                // The schema doesn't say which length is the outer one
                let shape: Vec<String> = dims.iter().map(ToString::to_string).collect();
                writeln!(out, "    /// {}, flattened", shape.join("x")).unwrap();
                format!("[{elem}; {count}]")
            }
        };

        writeln!(out, "    pub {field_name}: {field_type},").unwrap();

        writeln!(
            asserts,
            "    assert!(offset_of!({rust}, {field_name}) == {:#x});",
            field.offset
        )
        .unwrap();

        end = field.offset + schema[&field.kind].size * count;
    }

    if ty.size > end {
        writeln!(out, "    _unk_{end:x}: [u8; {:#x}],", ty.size - end).unwrap();
    }

    writeln!(out, "}}").unwrap();
    writeln!(out, "const _: () = {{").unwrap();
    writeln!(out, "    assert!(size_of::<{rust}>() == {:#x});", ty.size).unwrap();
    writeln!(out, "    assert!(align_of::<{rust}>() == {});", ty.align).unwrap();
    out.push_str(&asserts);
    writeln!(out, "}};").unwrap();
}

// Names and messages, a lone `Buffer` of char16 filling the type
fn is_text(ty: &Type) -> bool {
    match ty.fields.as_slice() {
        [field] => {
            field.name == "Buffer"
                && field.kind == "char16"
                && field.offset == 0
                && field.dims.len() == 1
                && ty.size == field.dims.iter().product::<usize>() * 2
        }
        _ => false,
    }
}

fn text_impl(rust: &str, out: &mut String) {
    writeln!(out, "impl SaveText for {rust} {{").unwrap();
    writeln!(out, "    fn units(&self) -> &[u16] {{").unwrap();
    writeln!(out, "        &self.buffer").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    fn units_mut(&mut self) -> &mut [u16] {{").unwrap();
    writeln!(out, "        &mut self.buffer").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
}

fn main() {
    println!("cargo::rerun-if-changed={SAVE}");

    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let path = dir.join(SAVE);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let schema = schema(&text);

    let mut types = HashMap::new();
    let mut seen = HashMap::new();

    for name in schema.keys() {
        if primitive(name).is_some() {
            continue;
        }

        let rust = type_ident(name, &schema);

        if let Some(other) = seen.insert(rust.clone(), name) {
            panic!("types `{name}` and `{other}` map to the same name");
        }

        types.insert(name.as_str(), rust);
    }

    let mut out = String::new();

    for (name, ty) in &schema {
        let Some(rust) = types.get(name.as_str()) else {
            continue;
        };

        if ty.fields.is_empty() {
            leaf(name, rust, ty, &mut out);
        } else {
            record(name, rust, ty, &schema, &types, &mut out);
        }

        if is_text(ty) {
            text_impl(rust, &mut out);
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    fs::write(out_dir.join("save.rs"), out).expect("write save.rs");
}
