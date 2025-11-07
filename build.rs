//! Build script for compiling Protocol Buffer definitions
//!
//! This script uses tonic-build to generate Rust code from .proto files.
//! The generated code is placed in the OUT_DIR and included via the
//! `tonic::include_proto!` macro in the grpc module.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the protobuf definitions
    // tonic_build will automatically put the output in OUT_DIR
    // which is accessible via tonic::include_proto!
    tonic_build::configure()
        .build_server(true) // Generate server code
        .build_client(true) // Generate client code for testing
        .compile(
            &["proto/memory_graph.proto"], // Proto files to compile
            &["proto"],                     // Include directories
        )?;

    // Re-run build script if proto files change
    println!("cargo:rerun-if-changed=proto/memory_graph.proto");

    Ok(())
}
