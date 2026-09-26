use sayall_windows::templates::MappingConfiguration;
use serde_json::Value;

#[test]
fn rust_serialization_matches_the_shared_builtin_catalog_contract() {
    let catalog = MappingConfiguration::default().template_catalog();
    let actual = serde_json::to_value(&catalog[..3]).expect("built-in catalog must serialize");
    let expected: Value = serde_json::from_str(include_str!(
        "../../../contracts/ipc/template-catalog-builtins.json"
    ))
    .expect("shared built-in catalog fixture must be valid JSON");

    assert_eq!(actual, expected);
}
