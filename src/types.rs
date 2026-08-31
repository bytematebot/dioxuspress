//! The data shapes generated code and a site's theme share.

/// One highlighted run of code, pre-colored for both themes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub text: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
}

/// Task list totals for one page, counted at build time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tasks {
    pub done: u32,
    pub total: u32,
}

impl Tasks {
    /// Completion as a whole percentage; `0` when the page has no task list.
    pub const fn percent(self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.done as u64 * 100) / self.total as u64) as u8
    }

    /// Whether the page has a task list at all.
    pub const fn any(self) -> bool {
        self.total > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TocItem {
    pub id: &'static str,
    pub title: &'static str,
    pub level: u8,
}

/// A sidebar entry. A group without its own `index.md` has `route: None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarItem {
    pub title: &'static str,
    pub route: Option<&'static str>,
    pub items: &'static [SidebarItem],
}

/// Per-page metadata, in reading order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageMeta {
    pub route: &'static str,
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub toc: &'static [TocItem],
    /// Prose with markup stripped, used by the search index.
    pub text: &'static str,
    /// Whether prev/next steps through this page.
    pub pager: bool,
    /// Task list totals, `Tasks::default()` on a page without one.
    pub tasks: Tasks,
}

/// Finds a page by route.
pub fn page<'a>(pages: &'a [PageMeta], route: &str) -> Option<&'a PageMeta> {
    pages.iter().find(|page| page.route == route)
}

/// Site-wide settings, surfaced to the theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteConfig {
    pub title: &'static str,
    pub description: Option<&'static str>,
    /// Shown in the navbar when set.
    pub repository: Option<&'static str>,
    /// Where the documentation index lives: `/` by default.
    pub docs_root: &'static str,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: "Documentation",
            description: None,
            repository: None,
            docs_root: "/",
        }
    }
}
