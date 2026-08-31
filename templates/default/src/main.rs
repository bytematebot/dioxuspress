mod components;
mod landing;

/// The site theme. Ordinary Dioxus components, yours to edit or replace.
mod theme;

/// The generated site. `dxpress` owns `generated/`; everything in `src/` is yours.
#[path = "../generated/docs.rs"]
mod docs;

use dioxus::prelude::*;
use docs::Route;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        dioxus_router::Router::<Route> {}
    }
}
