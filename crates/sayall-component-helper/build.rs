fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Applied by the loader before main: a current-user installation must
        // never resolve elevated Helper dependencies from its writable folder.
        println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:0x800");
    }
}
