fn main() {
    // iris-hub (siehe Cargo.toml, Feature "desktop-gui") baut ohne Tauri
    // selbst - tauri_build::build() erwartet dessen Config/Fenster-Setup und
    // schlägt sonst fehl ("missing `cargo:dev` instruction").
    if std::env::var_os("CARGO_FEATURE_DESKTOP_GUI").is_some() {
        tauri_build::build();
    }
}
