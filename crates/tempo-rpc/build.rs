#![allow(missing_docs)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    connectrpc_build::Config::new()
        .files(&["proto/tempo.proto"])
        .includes(&["proto"])
        .include_file("_connectrpc.rs")
        .compile()?;
    Ok(())
}
