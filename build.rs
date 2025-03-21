// use std::{fs, path::PathBuf};

fn main() {
    // println!("cargo::rustc-link-arg=-Tlinker.ld");

    // fs::copy("linker.ld", out_dir().join("linker.ld")).unwrap();

    // println!("cargo:rustc-link-search={}", out_dir().display());

    sparreal_macros::build_test_setup!();
}

// fn out_dir() -> PathBuf {
//     PathBuf::from(std::env::var("OUT_DIR").unwrap())
// }