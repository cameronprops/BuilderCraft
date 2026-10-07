//! Compile the contributor credits (contributors/*.tsv, craftrules standards/contributors.md)
//! into the About window. Missing or unreadable files give empty tables and a warning.

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contributors");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
    for f in ["commits.tsv", "prs.tsv", "models.tsv", "people.tsv"] {
        let p = root.join(f);
        println!("cargo::rerun-if-changed={}", p.display());
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
            println!("cargo::warning=contributors/{f}: {e}; the About window shows an empty table");
            String::new()
        });
        if let Err(e) = std::fs::write(out.join(f), text) {
            println!("cargo::warning=writing {f}: {e}");
        }
    }
}
