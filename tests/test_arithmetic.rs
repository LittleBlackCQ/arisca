use arisca::config::Config;
use clap::Parser;

#[test]
fn test_truncated_multiplier() {
    let cfg = Config::parse_from([
        "arisca",
        "examples/multiplier_truncated.aig",
        "--spec", "[63]*[65]",
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "63x65 truncated multiplier verification failed");
}

#[test]
fn test_adder() {
    let cfg = Config::parse_from([
        "arisca",
        "examples/adder.aig",
        "--spec", "[1]+[256]+[256]",
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "256-bit adder verification failed");
}

#[test]
fn test_mac() {
    let cfg = Config::parse_from([
        "arisca",
        "examples/mac.aig",
        "--spec", "[63]*[65]+[128]",
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "63x65 MAC verification failed");
}

#[test]
fn test_dotproduct() {
    let spec = concat!(
        "[8:0]*[8:128]+",
        "[8:8]*[8:136]+",
        "[8:16]*[8:144]+",
        "[8:24]*[8:152]+",
        "[8:32]*[8:160]+",
        "[8:40]*[8:168]+",
        "[8:48]*[8:176]+",
        "[8:56]*[8:184]+",
        "[8:64]*[8:192]+",
        "[8:72]*[8:200]+",
        "[8:80]*[8:208]+",
        "[8:88]*[8:216]+",
        "[8:96]*[8:224]+",
        "[8:104]*[8:232]+",
        "[8:112]*[8:240]+",
        "[8:120]*[8:248]",
    );
    let cfg = Config::parse_from([
        "arisca",
        "examples/dotproduct.aig",
        "--spec", spec,
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "dotproduct verification failed");
}

#[test]
fn test_iccad22a_test17() {
    let cfg = Config::parse_from([
        "arisca",
        "examples/iccad22a_test17.aig",
        "--spec", "o[33:0]=-[31:0]+[32:31]-2",
        "--delay",
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "iccad22A_test17 verification failed");
}

#[test]
fn test_divider() {
    let cfg = Config::parse_from([
        "arisca",
        "examples/divider.aig",
        "--spec", "[5] = o[5]*[5] + o[5]",
    ]);
    assert!(arisca::run(cfg).unwrap().is_zero(), "5-bit divider verification failed");
}
