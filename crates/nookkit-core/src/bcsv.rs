use core::ffi::c_void;
use std::sync::OnceLock;

use nookkit_bcsv::{PatchSet, Table};
use nookkit_sym::sym;
use nookkit_types::bcsv::BcsvMgr;
use nookkit_types::sead::{DirectResource, Heap, SafeString, VftDirectResource, VftHeap};

const FOLDER: &str = "content:/NookKit/Bcsv";

static PATCHES: OnceLock<PatchSet> = OnceLock::new();

/// Every patch, a broken one is reported and skipped
fn load() -> PatchSet {
    let mut set = PatchSet::default();
    let files = crate::content::json_files(FOLDER, "bcsv patch");

    for (name, json) in &files {
        if let Err(e) = set.add(name, json) {
            println!("[NookKit] bcsv patch {e}");
        }
    }

    if !set.is_empty() {
        println!(
            "[NookKit] {} patched tables from {} files",
            set.files().count(),
            files.len()
        );
    }
    set
}

fn patches() -> Option<&'static PatchSet> {
    Some(PATCHES.get_or_init(load)).filter(|set| !set.is_empty())
}

/// "Bcsv/ItemParam.bcsv" names the table "ItemParam"
fn table_name(path: &str) -> &str {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    name.strip_suffix(".bcsv").unwrap_or(name)
}

/// The table in `resource` with every patch for `file` applied, `None` when nothing changed
fn patched(patches: &PatchSet, file: &str, resource: &DirectResource) -> Option<Vec<u8>> {
    let Some(mut table) = Table::parse(unsafe { resource.data() }) else {
        println!("[NookKit] bcsv patch: {file} isn't a table");
        return None;
    };

    let applied = patches.apply(file, &mut table);
    for error in &applied.errors {
        println!("[NookKit] bcsv patch {error}");
    }
    if applied.added + applied.replaced == 0 {
        return None;
    }

    println!(
        "[NookKit] {file}: {} rows replaced, {} added",
        applied.replaced, applied.added
    );
    Some(table.to_bytes())
}

/// Swaps the resource's data for `bytes`, in the heap that holds its data, the one its destructor
/// frees an owned buffer from
unsafe fn replace_data(resource: &mut DirectResource, bytes: &[u8]) -> bool {
    let find_heap: unsafe extern "C" fn(*const c_void, *const c_void) -> *mut Heap = unsafe {
        let Ok(address) = nookkit_sym::fun_addr(&sym::sead::HeapMgr::findContainHeap) else {
            return false;
        };
        core::mem::transmute(address)
    };
    let Ok(instance) = nookkit_sym::data::<*const c_void>(&sym::sead::HeapMgr::sInstance) else {
        return false;
    };

    unsafe {
        let mgr = *instance;
        if mgr.is_null() {
            return false;
        }
        let heap = find_heap(mgr, resource.raw_data.cast());
        if heap.is_null() {
            return false;
        }

        let alignment = resource.get_load_data_alignment();
        let data = (*heap).try_alloc(bytes.len(), alignment).cast::<u8>();
        if data.is_null() {
            return false;
        }
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());

        if resource.setting_flag & 1 != 0 {
            (*heap).free(resource.raw_data.cast());
        }
        resource.raw_data = data;
        resource.raw_size = bytes.len() as u32;
        resource.buffer_size = bytes.len() as u32;
        resource.setting_flag |= 1;
    }
    true
}

// Every table reads its rows from the resource this returns, right after
nookkit_sym::hook! {
    pub load_resource = sym::game::BcsvMgr::loadResource;
    fn(mgr: *mut c_void, name: *const SafeString<'static>, heap: *mut Heap) -> *mut DirectResource {
        let resource = unsafe { original(mgr, name, heap) };

        let Some(resource_ref) = (unsafe { resource.as_mut() }) else {
            return resource;
        };
        let Some(patches) = patches() else {
            return resource;
        };
        let Some(file) = unsafe { name.as_ref().and_then(|n| n.as_str()) }
            .and_then(|path| patches.file(table_name(path)))
        else {
            return resource;
        };

        if let Some(bytes) = patched(patches, file, resource_ref)
            && !unsafe { replace_data(resource_ref, &bytes) }
        {
            println!("[NookKit] bcsv patch: no memory for the patched {file}");
        }
        resource
    }
}

fn loaded_tables() -> i32 {
    let Ok(instance) = nookkit_sym::data::<*const BcsvMgr>(&sym::game::BcsvMgr::sInstance) else {
        return 0;
    };
    unsafe { (*instance).as_ref() }.map_or(0, |mgr| mgr.table_count)
}

pub fn install() -> Result<(), nookkit_sym::SymError> {
    load_resource::install()?;

    // Reloading them instead would leave tables that share another's rows pointing at freed data.
    // The patches aren't read here, "content" may not be mounted yet
    let loaded = loaded_tables();
    if loaded > 0 {
        println!("[NookKit] bcsv: {loaded} tables loaded before NookKit started, unpatched");
    }
    Ok(())
}
