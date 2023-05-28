use std::io::Result;

fn main() -> Result<()> {
    // https://docs.rs/prost-build-config/latest/prost_build_config/
    let mut prost_build = prost_build::Config::new();
    prost_build.protoc_arg("--experimental_allow_proto3_optional");
    /*
    prost_build.type_attribute(
        "PricingData",
        "#[derive(serde::Serialize, serde::Deserialize)]",
    );*/
    prost_build
        .default_package_filename("gtfs")
        .compile_protos(&["src/gtfs/gtfs-realtime.proto"], &["src"])?;
    Ok(())
}
