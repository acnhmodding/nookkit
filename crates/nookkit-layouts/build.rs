// Generates layouts::<Layout> and pages::<Page> from the harvested layout data and the page export

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::{env, fs, path::PathBuf};

#[path = "../nookkit-sym/ident.rs"]
mod ident;

use ident::ident;

const PAGES: &str = "../../data/pages-3.0.3.tsv";
const LAYOUTS: &str = "../../data/layouts-3.0.3.toml";

#[derive(Default)]
struct Layout {
    control: String,
    parts: Vec<(String, String)>,
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    match s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => s.to_owned(),
    }
}

/// Only the subset tools/harvest_layouts.py writes: `[name]`, `control = "…"`, `parts.<pane> = "…"`
fn layouts(text: &str) -> BTreeMap<String, Layout> {
    let mut out: BTreeMap<String, Layout> = BTreeMap::new();
    let mut current = None;

    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let name = unquote(name);
            out.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }

        let (key, value) = line
            .split_once(" = ")
            .unwrap_or_else(|| panic!("{LAYOUTS}:{}: expected key = value", index + 1));
        let layout = current
            .as_ref()
            .and_then(|c| out.get_mut(c))
            .unwrap_or_else(|| panic!("{LAYOUTS}:{}: key outside a table", index + 1));

        match key.strip_prefix("parts.") {
            Some(pane) => layout.parts.push((unquote(pane), unquote(value))),
            None if key == "control" => layout.control = unquote(value),
            None => panic!("{LAYOUTS}:{}: unknown key `{key}`", index + 1),
        }
    }

    out
}

fn pages(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut cols = l.split('\t');
            let class = cols.next().unwrap_or_default().to_owned();
            let layout = cols.next().unwrap_or_default().to_owned();
            (class, layout)
        })
        .collect()
}

/// `L_BtnListEye_00` to `L_BTN_LIST_EYE_00`
fn screaming(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();

    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1);
        let hump = c.is_ascii_uppercase()
            && prev.is_some_and(|p| {
                p.is_ascii_lowercase()
                    || p.is_ascii_digit()
                    || (p.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase()))
            });
        if hump && !out.ends_with('_') {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }

    ident(&out)
}

fn kind(control: &str) -> &'static str {
    match control {
        "UISysScrollBtnList" => "crate::kind::List",
        "LpUniteButton" | "FixedButton" | "AppDragButton" | "ButtonNormal" | "ButtonCheck"
        | "ButtonRepeat" => "crate::kind::Button",
        _ => "crate::kind::Parts",
    }
}

fn consts(
    layout: &Layout,
    all: &BTreeMap<String, Layout>,
    root: bool,
    owner: &str,
    out: &mut String,
) {
    let mut seen = HashMap::new();

    for (pane, part) in &layout.parts {
        let name = screaming(pane);
        if let Some(other) = seen.insert(name.clone(), pane) {
            panic!("{owner}: panes `{pane}` and `{other}` map to the same constant");
        }

        let control = all.get(part).map_or("", |l| l.control.as_str());
        let path = if root {
            format!("/{pane}")
        } else {
            pane.clone()
        };
        let doc = if control.is_empty() {
            format!("Part layout `{part}`")
        } else {
            format!("`{control}`, part layout `{part}`")
        };
        writeln!(out, "    /// {doc}").unwrap();
        writeln!(
            out,
            "    pub const {name}: crate::PanePath<{}> = crate::PanePath::new({path:?}, {part:?});",
            kind(control)
        )
        .unwrap();
    }
}

fn main() {
    println!("cargo::rerun-if-changed={PAGES}");
    println!("cargo::rerun-if-changed={LAYOUTS}");

    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let all = layouts(&fs::read_to_string(dir.join(LAYOUTS)).expect("read the layout data"));
    let pages = pages(&fs::read_to_string(dir.join(PAGES)).expect("read the page export"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut out = String::new();
    let mut seen = HashMap::new();
    for (name, layout) in &all {
        let ty = ident(name);
        if let Some(other) = seen.insert(ty.clone(), name) {
            panic!("layouts `{name}` and `{other}` map to the same type");
        }

        let control = if layout.control.is_empty() {
            String::new()
        } else {
            format!(", control `{}`", layout.control)
        };
        writeln!(out, "/// Layout `{name}`{control}").unwrap();
        writeln!(out, "pub struct {ty};").unwrap();
        if !layout.parts.is_empty() {
            writeln!(out, "impl {ty} {{").unwrap();
            consts(layout, &all, false, name, &mut out);
            writeln!(out, "}}").unwrap();
        }
    }
    fs::write(out_dir.join("layouts.rs"), out).expect("write layouts.rs");

    let mut out = String::new();
    let mut seen = HashMap::new();
    for (class, layout) in &pages {
        let ty = ident(class.strip_suffix("Page").unwrap_or(class));
        if let Some(other) = seen.insert(ty.clone(), class) {
            panic!("pages `{class}` and `{other}` map to the same type");
        }

        writeln!(out, "/// `{class}`, layout `{layout}`").unwrap();
        writeln!(out, "pub struct {ty};").unwrap();
        writeln!(
            out,
            "impl crate::PageClass for {ty} {{\n    const CLASS: &'static str = {class:?};\n    const LAYOUT: &'static str = {layout:?};\n}}"
        )
        .unwrap();
        if let Some(root) = all.get(layout).filter(|l| !l.parts.is_empty()) {
            writeln!(out, "impl {ty} {{").unwrap();
            consts(root, &all, true, class, &mut out);
            writeln!(out, "}}").unwrap();
        }
    }
    fs::write(out_dir.join("pages.rs"), out).expect("write pages.rs");
}
