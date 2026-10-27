use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use nookkit_rstb::sizing::{GAME_SIZED, SHADER_ARCHIVE, Sizer, UNSUPPORTED, table_name};
use nookkit_rstb::table::{self, Table};
use nookkit_rstb::{declared_length, yaz0};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

// Live bytes now, restarting the peak from here
fn mark() -> usize {
    let live = LIVE.load(Ordering::Relaxed);
    PEAK.store(live, Ordering::Relaxed);
    live
}

fn peak_since(base: usize) -> usize {
    PEAK.load(Ordering::Relaxed).saturating_sub(base)
}

#[derive(Default)]
struct Memory {
    peak: usize,
    worst_ratio: f64,
    worst: String,
    // Largest peak of the files under 1 MiB of L + D, where a constant dominates
    small_peak: usize,
}

fn lengths(path: &str) -> (u64, u64) {
    let length = std::fs::metadata(path).map_or(0, |m| m.len());
    let mut prefix = [0u8; 18];
    let read = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut prefix))
        .unwrap_or(0);
    (length, declared_length(&prefix[..read]).unwrap_or(length))
}

fn files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, out);
        } else {
            out.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn main() {
    let root = std::env::args().nth(1).expect("usage: check <romfs>");
    let root = Path::new(&root);
    let table_base = mark();
    let file = std::fs::read(root.join(table::PATH)).unwrap();
    let data = yaz0::decompress(&file).unwrap();
    let table = Table::parse(&data).unwrap();
    assert_eq!(table.serialize(), data, "table round-trip");
    let start = Instant::now();
    let compressed = yaz0::compress(&data);
    println!(
        "Table: {} unique, {} collided; Yaz0 {} -> {} bytes in {:?} (vanilla {})",
        table.unique.len(),
        table.collided.len(),
        data.len(),
        compressed.len(),
        start.elapsed(),
        file.len()
    );
    assert_eq!(
        yaz0::decompress(&compressed).unwrap(),
        data,
        "Yaz0 round-trip"
    );

    println!("Table phase peak: {} KiB", peak_since(table_base) >> 10);

    let mut all = Vec::new();
    files(root, root, &mut all);
    all.sort();
    let mut sizer = Sizer::new(root.join(SHADER_ARCHIVE).to_string_lossy());
    let shader_base = mark();
    sizer.load_shader();
    println!(
        "Shader load: peak {} KiB, resident {} KiB",
        peak_since(shader_base) >> 10,
        (LIVE.load(Ordering::Relaxed) - shader_base) >> 10
    );
    let mut memory: BTreeMap<String, Memory> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut times: BTreeMap<String, (usize, Duration)> = BTreeMap::new();
    let mut slowest = (String::new(), Duration::ZERO);
    let start = Instant::now();
    for relative in &all {
        let Some(name) = table_name(relative) else {
            continue;
        };
        let path = root.join(relative).to_string_lossy().into_owned();
        let started = Instant::now();
        let base = mark();
        let size = sizer.file_size(&path, name);
        let peak = peak_since(base);
        let elapsed = started.elapsed();
        let kind = if name.starts_with("Model/") && !name.starts_with("Model/Layout_") {
            "model archive"
        } else {
            relative.rsplit('.').next().unwrap_or("")
        };
        let (length, decompressed) = lengths(&path);
        let input = (length + decompressed) as usize;
        let m = memory.entry(kind.to_string()).or_default();
        m.peak = m.peak.max(peak);
        if input < 1 << 20 {
            m.small_peak = m.small_peak.max(peak);
        } else if peak as f64 / input as f64 > m.worst_ratio {
            m.worst_ratio = peak as f64 / input as f64;
            m.worst = format!("{relative} (L {length}, D {decompressed}, peak {peak})");
        }
        let time = times.entry(kind.to_string()).or_default();
        time.0 += 1;
        time.1 += elapsed;
        if elapsed > slowest.1 {
            slowest = (relative.clone(), elapsed);
        }
        let vanilla = table.get(name).map(|e| e.size as i64);
        let outcome = match (size, vanilla) {
            (Err(e), _) => format!("error: {e}"),
            (Ok(UNSUPPORTED), _) => "unsupported".into(),
            (Ok(GAME_SIZED), None) => "game sized, no entry".into(),
            (Ok(GAME_SIZED), Some(_)) => "game sized, has entry".into(),
            (Ok(_), None) => "sized, no entry".into(),
            (Ok(s), Some(v)) if s == v => "exact".into(),
            (Ok(s), Some(v)) if s > v => "over".into(),
            (Ok(s), Some(v)) => {
                println!("UNDER {name}: {s} < {v}");
                "UNDER".into()
            }
        };
        *counts.entry(outcome).or_default() += 1;
    }
    println!("Sized {} files in {:?}", all.len(), start.elapsed());
    let mut times: Vec<_> = times.into_iter().collect();
    times.sort_by_key(|(_, (_, time))| std::cmp::Reverse(*time));
    for (kind, (count, time)) in times.iter().take(8) {
        println!("{time:>12.2?}  {count:>6}  {kind}");
    }
    println!("Slowest: {} in {:?}", slowest.0, slowest.1);
    if let Some(error) = sizer.shader_error() {
        println!("Shader archive failed: {error}");
    }
    println!("Memory per kind: peak, peak under 1 MiB input, worst peak / (L + D) over 1 MiB");
    for (kind, m) in &memory {
        println!(
            "{kind:>14}  {:>8} KiB  {:>8} KiB  {:.3}  {}",
            m.peak >> 10,
            m.small_peak >> 10,
            m.worst_ratio,
            m.worst
        );
    }
    for (outcome, count) in counts {
        println!("{count:>7}  {outcome}");
    }
}
