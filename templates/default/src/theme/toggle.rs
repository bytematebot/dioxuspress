//! Dark/light mode. The initial theme is applied by a blocking inline script, so a
//! dark-mode reader never sees a white flash.

use dioxus::prelude::*;

pub(crate) const INIT_THEME_JS: &str = r#"
(function () {
  try {
    // Only an explicit choice is stamped. Left alone, the CSS follows the system
    // setting by itself, which is what keeps the first paint correct.
    var stored = localStorage.getItem('dp-theme');
    if (stored === 'dark' || stored === 'light') {
      document.documentElement.setAttribute('data-theme', stored);
    }
  } catch (e) {}
})();
"#;

/// Softens the repaint when `data-theme` flips: a view transition where available, a
/// short-lived transition class otherwise.
const TOGGLE_JS: &str = r#"
(function () {
  var root = document.documentElement;

  // With no attribute set the page is following the system, so read what is painted.
  var current = root.getAttribute('data-theme');
  if (!current) {
    var prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
    current = prefersDark ? 'dark' : 'light';
  }
  var next = current === 'dark' ? 'light' : 'dark';

  var apply = function () {
    root.setAttribute('data-theme', next);
    try { localStorage.setItem('dp-theme', next); } catch (e) {}
  };

  var reduceMotion = window.matchMedia
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reduceMotion) {
    apply();
    return;
  }

  if (document.startViewTransition) {
    document.startViewTransition(apply);
    return;
  }

  root.classList.add('dp-theme-switching');
  apply();
  window.setTimeout(function () {
    root.classList.remove('dp-theme-switching');
  }, 260);
})();
"#;

/// Restores an explicitly chosen theme. Without one the CSS follows the system.
#[component]
pub fn ThemeStyles() -> Element {
    use_hook(|| {
        // `document::Script` re-emits on every render; eval once instead.
        let _ = super::THEME_CSS;
        document::eval(INIT_THEME_JS)
    });
    rsx! {}
}

/// Flips the theme and remembers the choice.
#[component]
pub fn ThemeToggle() -> Element {
    rsx! {
        button {
            class: super::ICON_BTN,
            r#type: "button",
            aria_label: "Toggle dark mode",
            title: "Toggle dark mode",
            onclick: move |_| {
                document::eval(TOGGLE_JS);
            },
            "◐"
        }
    }
}
