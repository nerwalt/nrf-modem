use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    #[cfg(not(any(feature = "nrf9151", feature = "nrf9160")))]
    compile_error!("No chip feature selected! Please enable either 'nrf9160' or 'nrf9151'.");

    #[cfg(feature = "nrf9160")]
    File::create(out_dir.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory-nrf9160.x"))
        .unwrap();

    #[cfg(feature = "nrf9151")]
    File::create(out_dir.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory-nrf9151.x"))
        .unwrap();

    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=memory-nrf9160.x");
    println!("cargo:rerun-if-changed=memory-nrf9151.x");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
