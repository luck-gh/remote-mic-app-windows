use sayall_windows::component_support::{helper_request, parse_helper_arguments};

fn main() {
    // This executable has not started threads. Never let elevated diagnostics
    // follow a caller-selected output path inherited through the environment.
    unsafe {
        std::env::remove_var("SAYALL_GATT_LOG");
    }
    if !sayall_windows::component_support::initialize_helper_diagnostics() {
        eprintln!("component_helper reason=audit_unavailable");
        std::process::exit(1);
    }
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((component, action)) = parse_helper_arguments(&arguments) else {
        eprintln!("component_helper reason=invalid_arguments");
        std::process::exit(64);
    };
    let result = helper_request(component, action);
    // The response contains enums and readiness facts only, never local paths.
    println!(
        "{}",
        serde_json::to_string(&result).expect("serialize component result")
    );
    std::process::exit(result.exit_code());
}
