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
    let builder = dioxus::LaunchBuilder::new();

    // `dxpress build --ssg` prerenders by asking the server for every static route and
    // requesting each one. The incremental renderer is what writes the resulting HTML,
    // and `public` is where the client build already put its assets.
    #[cfg(feature = "server")]
    let builder = builder.with_cfg(
        dioxus::server::ServeConfig::default().incremental(
            dioxus::server::IncrementalRendererConfig::default().static_dir("public"),
        ),
    );

    builder.launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        dioxus_router::Router::<Route> {}
    }
}

/// The route list `dx` fetches from `/api/static_routes` before prerendering.
#[cfg(feature = "server")]
#[server(endpoint = "static_routes")]
async fn static_routes() -> ServerFnResult<Vec<String>> {
    Ok(Route::static_routes().iter().map(ToString::to_string).collect())
}
