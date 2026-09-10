use sayall_windows::component_support::{helper_request, parse_helper_arguments};

fn main() {
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
