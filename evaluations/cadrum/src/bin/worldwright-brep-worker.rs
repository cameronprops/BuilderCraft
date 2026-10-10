//! One-shot out-of-process exact BRep command worker.
//! Reads one JSON request on stdin and writes one JSON result on stdout.
//! The parent should enforce an OS process timeout, memory policy, and error
//! handling around crashes from foreign native code.
fn main() {
    if let Err(error) = worldwright_cadrum_brep_evaluation::worker_protocol::serve_once() {
        eprintln!("WorldWright BRep worker I/O failure: {error}");
        std::process::exit(2);
    }
}
