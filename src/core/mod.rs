//! Markdown parsing, syntax highlighting, and filesystem-derived routing.

pub mod ast;
pub mod config;
pub mod directive;
pub mod frontmatter;
pub mod highlight;
pub mod lang;
pub mod parser;
pub mod site;
pub mod slug;
pub mod tag;

pub use ast::{Document, Frontmatter, HlToken, Node, PropValue, TocEntry};
pub use config::Config;
pub use parser::parse;
pub use site::{Page, SidebarNode, Site};
