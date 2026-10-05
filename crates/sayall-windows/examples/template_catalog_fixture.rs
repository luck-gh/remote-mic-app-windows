//! Emit the canonical built-in template catalog for the shared IPC fixture.

use sayall_windows::templates::MappingConfiguration;

fn main() {
    let catalog = MappingConfiguration::default().template_catalog();
    let builtins = &catalog[..3];
    println!(
        "{}",
        serde_json::to_string_pretty(builtins).expect("built-in catalog must serialize")
    );
}
