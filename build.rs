use std::env;
// Native linker script plus the version label the plugin logs and registers.
fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{dir}/link/switch32.ld");
    println!("cargo:rustc-link-arg=-Wl,--build-id=sha1");
    println!("cargo:rerun-if-changed=link/switch32.ld");
    println!("cargo:rerun-if-env-changed=MSG_VERSION");
    println!(
        "cargo:rustc-env=MSG_VERSION={}",
        env::var("MSG_VERSION").unwrap()
    );
}
