// Reads safe_clear.toml and emits SAFE_SIZES (see tools/compute-clear).

fn main() {
    let text = std::fs::read_to_string("safe_clear.toml").unwrap_or_else(|e| {
        panic!(
            "plugin/safe_clear.toml not found: {e}\n\
             Run the calibration tool first:\n\
             cargo run --manifest-path tools/compute-clear/Cargo.toml --release"
        )
    });

    let list = text
        .lines()
        .find(|l| l.starts_with("sizes"))
        .and_then(|l| l.split_once('['))
        .and_then(|(_, r)| r.split_once(']'))
        .map(|(inner, _)| inner)
        .expect("sizes array not found in safe_clear.toml");

    let sizes: Vec<u8> = list
        .split(',')
        .map(|s| s.trim().parse().expect("invalid number in safe_clear.toml"))
        .collect();

    assert_eq!(
        sizes.len(),
        40,
        "safe_clear.toml must have exactly 40 entries"
    );

    let out = format!(
        "pub(crate) const SAFE_SIZES: [u8; 40] = [{}];\n",
        sizes
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out_dir}/safe_clear.rs"), out).unwrap();
    println!("cargo:rerun-if-changed=safe_clear.toml");
}
