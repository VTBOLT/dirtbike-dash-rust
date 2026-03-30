fn main() {
    if std::env::var("CARGO_FEATURE_RELEASE").is_ok() {
        let config = slint_build::CompilerConfiguration::new();
        slint_build::compile_with_config("./ui/main.slint", config).unwrap();
    }
}