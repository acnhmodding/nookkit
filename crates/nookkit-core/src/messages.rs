use core::ffi::{CStr, c_char, c_void};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use nookkit_msbt::{Attribute, Entry, PatchSet};
use nookkit_sym::sym;
use nookkit_types::lp::ui::MessageFile;
use nookkit_types::sead::SafeString;

// The game reads messages only once "content" is mounted
const FOLDER: &str = "content:/NookKit/Message";

static PATCHES: OnceLock<PatchSet> = OnceLock::new();
static LOCALE: OnceLock<String> = OnceLock::new();

// File handles (MessageArchive::findFile's {msbt*, count} entries) of patched files, by address
static HANDLES: Mutex<Option<HashMap<usize, &'static str>>> = Mutex::new(None);

fn load() -> PatchSet {
    let mut set = PatchSet::default();

    let files = match crate::content::json_files(FOLDER) {
        Ok(files) => files,
        Err(e) => {
            println!("[NookKit] message patches: {e}");
            return set;
        }
    };

    let mut read = 0;

    for (name, json) in files {
        let json = match json {
            Ok(json) => json,
            Err(e) => {
                println!("[NookKit] message patch {name}: {e}");
                continue;
            }
        };

        read += 1;

        match set.add(&name, &json) {
            Ok(replaced) => {
                for label in replaced {
                    println!("[NookKit] message patch {name} replaces {label}");
                }
            }
            Err(e) => println!("[NookKit] message patch {e}"),
        }
    }

    if !set.is_empty() {
        println!(
            "[NookKit] {} patched messages from {} files",
            set.len(),
            read
        );
    }

    set
}

fn patches() -> Option<&'static PatchSet> {
    PATCHES.get_or_init(load);

    PATCHES.get().filter(|set| !set.is_empty())
}

// The game's language as its archives name it, read on first use when the game has settled it
fn locale() -> Option<&'static str> {
    if let Some(locale) = LOCALE.get() {
        return Some(locale);
    }

    let language: unsafe extern "C" fn() -> u32 = unsafe {
        core::mem::transmute(nookkit_sym::fun_addr(&sym::game::System::getLanguageImpl).ok()?)
    };

    let codes =
        nookkit_sym::data::<[*const c_char; 16]>(&sym::game::System::sLanguageCodes).ok()?;

    let code = unsafe { *(*codes).get(language() as usize)? };

    if code.is_null() {
        return None;
    }

    let locale = unsafe { CStr::from_ptr(code) }.to_str().ok()?.to_owned();

    Some(LOCALE.get_or_init(|| locale))
}

fn file_of(handle: *const c_void) -> Option<&'static str> {
    HANDLES
        .lock()
        .ok()?
        .as_ref()?
        .get(&(handle as usize))
        .copied()
}

fn handle_of(file: &str) -> Option<usize> {
    let handles = HANDLES.lock().ok()?;

    handles
        .as_ref()?
        .iter()
        .find(|(_, f)| **f == file)
        .map(|(&handle, _)| handle)
}

unsafe fn str_of<'a>(string: *const SafeString<'a>) -> Option<&'a str> {
    unsafe { string.as_ref()?.as_str() }
}

fn patched_text(handle: *const c_void, label: *const c_char) -> Option<&'static [u16]> {
    let patches = patches()?;
    let file = file_of(handle)?;
    let label = unsafe { CStr::from_ptr(label) }.to_str().ok()?;

    patches.get(file, label)?.text_for(locale()?)
}

// The archive entry for "<file>.msbt", which every reader looks up before reading a label
nookkit_sym::hook! {
    pub find_file = sym::Lp::UI::MessageArchive::findFile;
    fn(archive: *mut c_void, name: *const SafeString<'static>) -> *mut c_void {
        let handle = unsafe { original(archive, name) };

        if handle.is_null() {
            return handle;
        }

        let Some(patches) = patches() else {
            return handle;
        };

        let file = unsafe { str_of(name) }.and_then(|n| n.strip_suffix(".msbt"));

        if let Ok(mut handles) = HANDLES.lock() {
            let handles = handles.get_or_insert_with(HashMap::new);

            match file.and_then(|f| patches.file(f)) {
                Some(file) => {
                    handles.insert(handle as usize, file);
                }
                // An address reused by another file after an archive reload
                None => {
                    handles.remove(&(handle as usize));
                }
            }
        }

        handle
    }
}

nookkit_sym::hook! {
    pub has_label = sym::Lp::UI::MessageFile::hasLabel;
    fn(handle: *mut c_void, label: *mut c_char) -> bool {
        patched_text(handle, label).is_some() || unsafe { original(handle, label) }
    }
}

#[repr(C)]
struct Text {
    units: *const u16,
    len: i32,
}

pub mod find_text {
    use core::arch::naked_asm;

    use super::*;

    static ORIGINAL: AtomicUsize = AtomicUsize::new(0);

    // (handle, label) returning a Text through x8, which Rust can't name, so it's passed on as a
    // third argument
    #[unsafe(naked)]
    extern "C" fn entry() {
        naked_asm!("mov x2, x8", "b {find}", find = sym find);
    }

    #[unsafe(naked)]
    unsafe extern "C" fn call(
        handle: *const c_void,
        label: *const c_char,
        out: *mut Text,
        original: usize,
    ) {
        naked_asm!("mov x8, x2", "br x3");
    }

    extern "C" fn find(handle: *const c_void, label: *const c_char, out: *mut Text) {
        match patched_text(handle, label) {
            Some(units) => unsafe {
                out.write(Text {
                    units: units.as_ptr(),
                    len: units.len() as i32,
                });
            },
            None => unsafe { call(handle, label, out, ORIGINAL.load(Ordering::Acquire)) },
        }
    }

    pub fn install() -> Result<(), nookkit_sym::SymError> {
        let target = nookkit_sym::fun_addr(&sym::Lp::UI::MessageFile::findText)?;

        unsafe {
            nookkit_sym::__hook_function(
                target as *const _,
                entry as *const _,
                ORIGINAL.as_ptr().cast(),
            );
        }

        if ORIGINAL.load(Ordering::Acquire) == 0 {
            return Err(nookkit_sym::SymError::HookFailed(target));
        }

        Ok(())
    }
}

unsafe fn attribute_size(handle: usize) -> usize {
    let Some(file) = (unsafe { (handle as *const MessageFile).as_ref() }) else {
        return 0;
    };

    let Some(binary) = (unsafe { file.binary.as_ref() }) else {
        return 0;
    };

    unsafe { binary.attribute_size() }
}

type ReadAttribute =
    unsafe fn(*mut c_void, *mut u8, *const SafeString<'static>, *const SafeString<'static>) -> bool;

// One of MessageAttribute's readers for a patched label: its own attributes, another label's, or
// zeros for a new label without any. `None` leaves the read to the game
fn read_attribute(
    original: ReadAttribute,
    reader: *mut c_void,
    dst: *mut u8,
    file: *const SafeString<'static>,
    label: *const SafeString<'static>,
) -> Option<bool> {
    let patches = patches()?;
    let file_name = unsafe { str_of(file) }?;
    let entry: &Entry = patches.get(file_name, unsafe { str_of(label) }?)?;
    entry.text_for(locale()?)?;

    // The game's own read finds the file and fills dst when the label is already there
    let found = unsafe { original(reader, dst, file, label) };
    let size = unsafe { attribute_size(handle_of(file_name)?) };

    match &entry.attribute {
        Some(Attribute::Like(other)) => {
            let other = std::ffi::CString::new(other.as_str()).ok()?;
            let other = SafeString::new(&other).ok()?;
            // Only read during the call
            let other = (&raw const other).cast::<SafeString<'static>>();
            Some(unsafe { original(reader, dst, file, other) })
        }
        Some(Attribute::Bytes(bytes)) if bytes.len() == size => {
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, size) };
            Some(true)
        }
        Some(Attribute::Bytes(bytes)) => {
            println!(
                "[NookKit] message patch {}: {file_name} attributes are {} bytes, not {size}",
                entry.source,
                bytes.len()
            );

            Some(found)
        }
        None if found => Some(true),
        None => {
            unsafe { core::ptr::write_bytes(dst, 0, size) };
            Some(true)
        }
    }
}

macro_rules! attribute_readers {
    ($($name:ident = $symbol:path;)*) => {
        $(
            nookkit_sym::hook! {
                pub $name = $symbol;
                fn(reader: *mut c_void, dst: *mut u8, file: *const SafeString<'static>, label: *const SafeString<'static>) -> bool {
                    read_attribute(original, reader, dst, file, label)
                        .unwrap_or_else(|| unsafe { original(reader, dst, file, label) })
                }
            }
        )*

        fn install_attribute_readers() -> Result<(), nookkit_sym::SymError> {
            $($name::install()?;)*
            Ok(())
        }
    };
}

attribute_readers! {
    read_attribute1 = sym::game::MessageAttribute::read1;
    read_attribute2 = sym::game::MessageAttribute::read2;
    read_attribute3 = sym::game::MessageAttribute::read3;
    read_attribute5 = sym::game::MessageAttribute::read5;
    read_attribute6 = sym::game::MessageAttribute::read6;
    read_attribute7 = sym::game::MessageAttribute::read7;
}

pub fn install() -> Result<(), nookkit_sym::SymError> {
    find_file::install()?;
    has_label::install()?;
    find_text::install()?;

    install_attribute_readers()
}
