//! Opt-in external OpenSCAD CLI adapter; no OpenSCAD source linked.
//! Opens no documents and executes nothing without explicit CLI consent.
//! This is NOT a security sandbox: trusted SCAD scripts may include/use files.
//! OS-level process sandboxing is a separate dependency before UI auto-preview.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const USAGE: &str = "usage: cadcraft-cli openscad-render --allow-execute --bin /absolute/path/to/openscad INPUT.scad OUTPUT.(stl|3mf)";
const MAX_OUTPUT_BYTES: u64 = 128 * 1024 * 1024;

struct Request<'a> {
    executable: &'a str,
    input: &'a str,
    output: &'a str,
}
fn parse(args: &[String]) -> Result<Request<'_>, String> {
    match args {
        [confirm, flag, exe, input, output] if confirm == "--allow-execute" && flag == "--bin" => Ok(Request { executable: exe, input, output }),
        _ => Err(USAGE.into()),
    }
}
fn extension(path: &Path, wanted: &str) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
}
fn validated(request: &Request<'_>) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let exe = std::fs::canonicalize(request.executable).map_err(|e| format!("OpenSCAD binary: {e}"))?;
    if !exe.is_file() {
        return Err("OpenSCAD executable must be an existing regular file".into());
    }
    let source = std::fs::canonicalize(request.input).map_err(|e| format!("OpenSCAD input: {e}"))?;
    if !source.is_file() || !extension(&source, "scad") {
        return Err("input must be an existing .scad source file".into());
    }
    let out = Path::new(request.output);
    if !(extension(out, "stl") || extension(out, "3mf")) {
        return Err("OpenSCAD output must be an .stl or .3mf file".into());
    }
    if out.exists() {
        return Err("refusing to overwrite an existing output model".into());
    }
    let name = out.file_name().ok_or("output filename required")?;
    let parent = out.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let parent = std::fs::canonicalize(parent).map_err(|e| format!("output folder: {e}"))?;
    let output = parent.join(name);
    if output == source || output == exe {
        return Err("output collides with input or program".into());
    }
    Ok((exe, source, output))
}
pub fn render(args: &[String]) -> Result<(), String> {
    let request = parse(args)?;
    let (binary, source, output) = validated(&request)?;
    let mut child = Command::new(binary)
        .arg("--hardwarnings")
        .arg("-o")
        .arg(&output)
        .arg(&source)
        .current_dir(source.parent().ok_or("SCAD parent directory required")?)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("could not launch configured OpenSCAD renderer: {e}"))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| format!("OpenSCAD process status: {e}"))? {
            Some(status) => break status,
            None if started.elapsed() > Duration::from_secs(120) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&output);
                return Err("OpenSCAD render timed out (120 seconds)".into());
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    if !status.success() {
        let _ = std::fs::remove_file(&output);
        return Err(format!("OpenSCAD renderer failed with status {status}"));
    }
    let length = std::fs::metadata(&output).map_err(|e| format!("OpenSCAD output missing: {e}"))?.len();
    if length == 0 || length > MAX_OUTPUT_BYTES {
        let _ = std::fs::remove_file(&output);
        return Err("OpenSCAD output empty or exceeds 128 MiB limit".into());
    }
    println!("OpenSCAD rendered {} ({} bytes)", output.display(), length);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_consent_is_required_before_opening_a_script() {
        let args = ["--bin", "openscad", "model.scad", "out.stl"].map(str::to_string);
        assert!(parse(&args).is_err());
        let args = ["--allow-execute", "--bin", "/usr/bin/openscad", "model.scad", "out.stl"].map(str::to_string);
        let parsed = parse(&args).unwrap();
        assert_eq!(parsed.input, "model.scad");
    }
    #[test]
    fn only_supported_explicit_output_formats() {
        assert!(extension(Path::new("test.STL"), "stl"));
        assert!(extension(Path::new("test.3mf"), "3mf"));
        assert!(!extension(Path::new("test.scad"), "stl"));
        assert!(!extension(Path::new("test.exe"), "3mf"));
    }
}
