//! Typed page identities and parts pane paths, generated from `data/pages-3.0.3.tsv` (Ghidra export) and
//! `data/layouts-3.0.3.toml` (romfs harvest). A path's kind comes from its part layout's cnt1 control

use std::borrow::Cow;
use std::marker::PhantomData;

/// What a [`PanePath`] points at, from its part layout's cnt1 control
pub mod kind {
    /// Any parts pane
    pub struct Parts;
    /// A button control (LpUniteButton, FixedButton, …)
    pub struct Button;
    /// A UISysScrollBtnList
    pub struct List;
}

/// Anything naming a pane, a typed [`PanePath`] or a string for panes outside the generated layouts
pub trait PaneRef {
    fn path(&self) -> &str;
}

impl PaneRef for str {
    fn path(&self) -> &str {
        self
    }
}

impl<T: PaneRef + ?Sized> PaneRef for &T {
    fn path(&self) -> &str {
        T::path(self)
    }
}

impl PaneRef for String {
    fn path(&self) -> &str {
        self
    }
}

impl<K> PaneRef for PanePath<K> {
    fn path(&self) -> &str {
        PanePath::path(self)
    }
}

/// A scroll list path. Typed paths must be [`kind::List`] panes, strings are accepted for lists a mod adds
/// in its own layout
///
/// ```
/// fn list(_: impl nookkit_layouts::ListRef) {}
///
/// list(nookkit_layouts::pages::CharaMake::L_BTN_LIST_EYE_00);
/// list("/L_BtnListMouth_00");
/// ```
///
/// ```compile_fail,E0277
/// fn list(_: impl nookkit_layouts::ListRef) {}
///
/// list(nookkit_layouts::pages::CharaMake::L_BTN_OK_00);
/// ```
pub trait ListRef: PaneRef {}

impl ListRef for str {}
impl<T: ListRef + ?Sized> ListRef for &T {}
impl ListRef for String {}
impl ListRef for PanePath<kind::List> {}

/// A page class the game builds, keyed by its runtime class name (`getClassInfo`)
pub trait PageClass {
    const CLASS: &'static str;
    /// The page's root layout
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
    pub fn then<K2>(&self, child: RelPath<K2>) -> PanePath<K2> {
        PanePath {
            path: Cow::Owned(format!("{}/{}", self.path, child.name)),
            layout: child.layout,
            kind: PhantomData,
        }
    }
}

/// A parts pane inside a part layout, only usable through [`PanePath::then`] since the same part layout
/// appears at many places
///
/// ```compile_fail,E0277
/// fn pane(_: impl nookkit_layouts::PaneRef) {}
///
/// pane(nookkit_layouts::layouts::CharaMake::L_BTN_OK_00);
/// ```
pub struct RelPath<K> {
    name: &'static str,
    layout: &'static str,
    kind: PhantomData<K>,
}

impl<K> RelPath<K> {
    #[doc(hidden)]
    pub const fn new(name: &'static str, layout: &'static str) -> Self {
        Self {
            name,
            layout,
            kind: PhantomData,
        }
    }

    /// The pane's name within its part layout
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The part layout this pane instantiates
    pub fn layout(&self) -> &'static str {
        self.layout
    }
}

impl<K> Clone for RelPath<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K> Copy for RelPath<K> {}

impl<K> core::fmt::Debug for RelPath<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("RelPath").field(&self.name).finish()
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

/// One zero-sized struct per layout, holding its parts panes as [`RelPath`]s for [`PanePath::then`]
#[allow(non_camel_case_types)]
pub mod layouts {
    include!(concat!(env!("OUT_DIR"), "/layouts.rs"));
}

/// One marker per page class, holding its root layout's parts panes as page paths
#[allow(non_camel_case_types)]
pub mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages.rs"));
}
