extern crate cc;

fn main() -> Result<(), String> {
    println!("cargo:rerun-if-changed=./aiger");
    cc::Build::new()
        .include("aiger")
        .file("aiger/aiger.c")
        .opt_level(3)
        .warnings(false)
        .compile("aiger");
    Ok(())
}
