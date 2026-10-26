//! Typed page identities and parts pane paths, generated from `data/pages-3.0.3.tsv` (Ghidra export) and
//! `data/layouts-3.0.3.toml` (romfs harvest). A path's kind comes from its part layout's cnt1 control

use std::borrow::Cow;
use std::marker::PhantomData;

/// Any parts pane
pub struct Parts;
/// A button control (LpUniteButton, FixedButton, …)
pub struct Button;
/// A UISysScrollBtnList
pub struct List;

/// A page class the game builds, keyed by its runtime class name (`getClassInfo`)
pub trait PageClass {
    const CLASS: &'static str;
    /// The page's root layout, empty for the one page without a layout archive
    const LAYOUT: &'static str;
}

/// A parts pane path, typed by the control its part layout declares
pub struct PanePath<K> {
    path: Cow<'static, str>,
    layout: &'static str,
    kind: PhantomData<K>,
}

impl<K> PanePath<K> {
    #[doc(hidden)]
    pub const fn new(path: &'static str, layout: &'static str) -> Self {
        Self {
            path: Cow::Borrowed(path),
            layout,
            kind: PhantomData,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// The part layout this pane instantiates
    pub fn layout(&self) -> &'static str {
        self.layout
    }

    /// A parts pane inside this one, e.g. `page_pane.then(layouts::LBtnList::L_BTN_00)`
    pub fn then<K2>(&self, child: PanePath<K2>) -> PanePath<K2> {
        PanePath {
            path: Cow::Owned(format!("{}/{}", self.path, child.path)),
            layout: child.layout,
            kind: PhantomData,
        }
    }
}

impl<K> Clone for PanePath<K> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            layout: self.layout,
            kind: PhantomData,
        }
    }
}

impl<K> core::fmt::Debug for PanePath<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("PanePath").field(&self.path).finish()
    }
}

/// One zero-sized struct per layout, holding its parts panes as paths relative to that layout
#[allow(non_camel_case_types)]
pub mod layouts {
    include!(concat!(env!("OUT_DIR"), "/layouts.rs"));
}

/// One marker per page class, holding its root layout's parts panes as page paths
#[allow(non_camel_case_types)]
pub mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages.rs"));
}
