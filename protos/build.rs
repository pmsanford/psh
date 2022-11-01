fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = prost_build::Config::default();

    tonic_build::configure()
        .build_server(true)
        .compile_with_config(config, &["proto/env.proto"], &["proto"])?;
    Ok(())
}
