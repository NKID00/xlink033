use std::{env, path::PathBuf};

fn main() {
    println!("cargo::rustc-link-arg-bins=--nmagic");
    println!(
        "cargo::rustc-link-search={}",
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).display()
    );
    println!("cargo::rustc-link-arg-bins=-Tlink.x");
}
