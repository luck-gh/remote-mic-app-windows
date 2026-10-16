fn main() {
    let manifest_dir =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let repository_root = manifest_dir.parent().unwrap_or(&manifest_dir);
    let identity_script = repository_root.join("scripts/build-identity.ps1");
    let packaging_build = std::env::var("SAYALL_BUNDLE_BUILD").as_deref() == Ok("1");
    let mut identity_command = std::process::Command::new("powershell.exe");
    identity_command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&identity_script)
        .args(["-RepositoryRoot"])
        .arg(repository_root)
        .args(["-WriteManifest", "-CargoFormat"]);
    if packaging_build {
        identity_command.args(["-IncludePayloads"]);
    }
    let identity_result = identity_command.output();
    let identity_failure = match &identity_result {
        Err(error) => Some(format!(
            "helper launch failed (kind={:?}, os_code={:?})",
            error.kind(),
            error.raw_os_error()
        )),
        Ok(output) if !output.status.success() => Some(format!(
            "helper exited unsuccessfully (exit_code={:?}, stderr_bytes={})",
            output.status.code(),
            output.stderr.len()
        )),
        Ok(output) if String::from_utf8(output.stdout.clone()).is_err() => {
            Some("helper output was not valid UTF-8".to_owned())
        }
        _ => None,
    };
    let identity_output = identity_result
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok());
    if packaging_build && identity_output.is_none() {
        panic!(
            "Could not establish build identity for a bundle build: {}",
            identity_failure
                .as_deref()
                .unwrap_or("identity output unavailable")
        );
    }
    let parsed_identity = identity_output.as_deref().map(|output| {
        output
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect::<std::collections::HashMap<_, _>>()
    });
    let revision = parsed_identity
        .as_ref()
        .and_then(|identity| {
            identity
                .get("source_base_commit")
                .map(|value| (*value).to_owned())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|revision| revision.trim().to_owned())
                .filter(|revision| revision.len() == 40)
                .unwrap_or_else(|| "unknown".to_owned())
        });
    println!("cargo:rustc-env=SAYALL_SOURCE_REVISION={revision}");
    let fingerprint = parsed_identity
        .as_ref()
        .and_then(|identity| identity.get("source_fingerprint").copied())
        .unwrap_or("unknown");
    println!("cargo:rustc-env=SAYALL_SOURCE_FINGERPRINT={fingerprint}");
    let state = parsed_identity
        .as_ref()
        .and_then(|identity| identity.get("source_state").copied())
        .unwrap_or("unknown");
    println!("cargo:rustc-env=SAYALL_SOURCE_STATE={state}");
    let channel = parsed_identity
        .as_ref()
        .and_then(|identity| identity.get("build_channel").copied())
        .unwrap_or("unknown");
    println!("cargo:rustc-env=SAYALL_BUILD_CHANNEL={channel}");
    let payload_inputs = parsed_identity
        .as_ref()
        .and_then(|identity| identity.get("payload_inputs").copied())
        .unwrap_or("unknown");
    println!("cargo:rustc-env=SAYALL_PAYLOAD_INPUTS={payload_inputs}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");
    println!("cargo:rerun-if-changed=../.git/packed-refs");
    println!("cargo:rerun-if-env-changed=SAYALL_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SAYALL_SOURCE_FINGERPRINT");
    println!("cargo:rerun-if-env-changed=SAYALL_SOURCE_STATE");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_EVENT_NAME");
    println!("cargo:rerun-if-env-changed=SAYALL_BUNDLE_BUILD");
    println!("cargo:rerun-if-changed=../scripts/build-identity.ps1");
    for input_dir in [
        "../src",
        "../src-tauri",
        "../crates",
        "../hardware",
        "../scripts",
        "../public",
        "../contracts",
    ] {
        println!("cargo:rerun-if-changed={input_dir}");
    }
    for input_file in [
        "../Cargo.toml",
        "../Cargo.lock",
        "../package.json",
        "../pnpm-lock.yaml",
        "../pnpm-workspace.yaml",
        "../index.html",
        "../vite.config.ts",
    ] {
        println!("cargo:rerun-if-changed={input_file}");
    }
    println!("cargo:rerun-if-env-changed=SAYALL_RELEASE_TAG");
    tauri_build::build()
}
