fn main() {
    println!("cargo:rerun-if-changed=protos");
    protobuf_codegen::Codegen::new()
        // All inputs and imports from the inputs must reside in `includes` directories.
        .includes(["protos"])
        // Inputs must reside in some of include paths.
        .input("protos/metrics.proto")
        // Specify output directory relative to Cargo output directory.
        .out_dir("src/protos")
        .run_from_script();
}
