fn main() {
    // Only the `release` feature pulls in the Slint UI, so only compile the
    // .slint files then. Other builds (sim, soc, plain) skip this entirely.
    #[cfg(feature = "release")]
    {
        let config = slint_build::CompilerConfiguration::new();
        slint_build::compile_with_config("./ui/main.slint", config).unwrap();
    }
}
