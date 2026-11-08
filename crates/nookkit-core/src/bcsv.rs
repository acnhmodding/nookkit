use std::sync::OnceLock;

use nookkit_formats::bcsv::{PatchSet, Plan, Table, WriteError};
use nookkit_sym::sym;
use nookkit_types::bcsv::BcsvMgr;
use nookkit_types::native;
use nookkit_types::sead::{DirectResource, Heap, HeapMgr, SafeString, VftDirectResource, VftHeap};

const FOLDER: &str = "content:/NookKit/Bcsv";

static PATCHES: OnceLock<PatchSet> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
enum BcsvError {
    #[error(transparent)]
    Native(#[from] native::Error),

    #[error("sead has no heap manager")]
    NoHeapMgr,

    #[error("no heap holds the table")]
    NoHeap,

    #[error("no memory for the {size} byte patched table")]
    OutOfMemory { size: usize },

    #[error(transparent)]
    Write(#[from] WriteError),
}

fn load() -> PatchSet {
    let mut set = PatchSet::default();

    let files = match crate::content::json_files(FOLDER) {
        Ok(files) => files,
        Err(e) => {
            println!("[NookKit] bcsv patches: {e}");
            return set;
        }
    };

    let mut read = 0;

    for (name, json) in files {
        let json = match json {
            Ok(json) => json,
            Err(e) => {
                println!("[NookKit] bcsv patch {name}: {e}");
                continue;
            }
        };

        read += 1;

        if let Err(e) = set.add(&name, &json) {
            println!("[NookKit] bcsv patch {e}");
        }
    }

    if !set.is_empty() {
        println!(
            "[NookKit] {} patched tables from {} files",
            set.files().count(),
            read
        );
    }

    set
}

fn patches() -> Option<&'static PatchSet> {
    Some(PATCHES.get_or_init(load)).filter(|set| !set.is_empty())
}

fn table_name(path: &str) -> &str {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);

    name.strip_suffix(".bcsv").unwrap_or(name)
}

// The heap that holds the resource's data, the one its destructor frees an owned buffer from
unsafe fn data_heap(resource: &DirectResource) -> Result<*mut Heap, BcsvError> {
    let mgr = unsafe { HeapMgr::instance()?.as_mut() }.ok_or(BcsvError::NoHeapMgr)?;
    let heap = unsafe { mgr.find_contain_heap(resource.raw_data)? };

    if heap.is_null() {
        return Err(BcsvError::NoHeap);
    }

    Ok(heap)
}

struct Patched {
    heap: *mut Heap,
    data: *mut u8,
    size: usize,
}

impl Patched {
    // The game leaves `malloc` too little memory for a copy of a large table
    unsafe fn write(resource: &DirectResource, plan: &Plan) -> Result<Self, BcsvError> {
        let heap = unsafe { data_heap(resource)? };
        let size = plan.size();

        unsafe {
            let alignment = resource.get_load_data_alignment();
            let data = (*heap).try_alloc(size, alignment).cast::<u8>();

            if data.is_null() {
                return Err(BcsvError::OutOfMemory { size });
            }

            if let Err(e) = plan.write(core::slice::from_raw_parts_mut(data, size)) {
                (*heap).free(data.cast());
                return Err(e.into());
            }

            Ok(Self { heap, data, size })
        }
    }

    // Gives the table to the resource, once nothing reads its old data
    unsafe fn hand_to(self, resource: &mut DirectResource) {
        if resource.owns_data() {
            unsafe { (*self.heap).free(resource.raw_data.cast()) };
        }

        resource.set_owned_data(self.data, self.size as u32);
    }
}

// Every table reads its rows from the resource this returns, right after
nookkit_sym::hook! {
    pub load_resource = sym::game::BcsvMgr::loadResource;
    fn(mgr: *mut BcsvMgr, name: *const SafeString<'static>, heap: *mut Heap) -> *mut DirectResource {
        let resource = unsafe { original(mgr, name, heap) };

        let Some(resource_ref) = (unsafe { resource.as_mut() }) else {
            return resource;
        };

        let Some(patches) = patches() else {
            return resource;
        };

        let Some(path) = (unsafe { name.as_ref().and_then(|n| n.as_str()) }) else {
            return resource;
        };

        let Some(file) = patches.file(table_name(path)) else {
            return resource;
        };

        // Not borrowed from the resource, whose data is replaced once the table is dropped
        let data = unsafe { core::slice::from_raw_parts(resource_ref.raw_data, resource_ref.raw_size as usize) };

        let table = match Table::parse(data) {
            Ok(table) => table,
            Err(e) => {
                println!("[NookKit] bcsv patch: {file}: {e}");
                return resource;
            }
        };

        let plan = patches.plan(file, &table);

        for skipped in &plan.skipped {
            println!("[NookKit] bcsv patch {skipped}");
        }

        if plan.is_empty() {
            return resource;
        }

        let (replaced, added) = (plan.replaced, plan.added);
        let patched = unsafe { Patched::write(resource_ref, &plan) };
        drop(plan);
        drop(table);

        match patched {
            Ok(patched) => {
                unsafe { patched.hand_to(resource_ref) };
                println!("[NookKit] {file}: {replaced} rows replaced, {added} added");
            }
            Err(e) => println!("[NookKit] bcsv patch {file}: {e}"),
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

    // Reloading them instead would leave tables that share another's rows pointing at freed data
    // The patches aren't read here, "content" may not be mounted yet
    let loaded = loaded_tables();

    if loaded > 0 {
        println!("[NookKit] bcsv: {loaded} tables loaded before NookKit started, unpatched");
    }

    Ok(())
}
