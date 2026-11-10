use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(output.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("../memory.x"))
        .unwrap();

    println!("cargo:rustc-link-search={}", output.display());
    println!("cargo:rerun-if-changed=../memory.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
