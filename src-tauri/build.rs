fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Usa protoc binario incluido — sin instalación manual requerida
    let protoc = protoc_bin_vendored::protoc_bin_path().unwrap();
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/vfpsync.proto"], &["proto"])?;

    tauri_build::build();

    Ok(())
}
