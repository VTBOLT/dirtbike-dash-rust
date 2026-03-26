#[cfg(feature = "release")]
fn main() {
    let mut config = slint_build::CompilerConfiguration::new();
    slint_build::compile_with_config("./ui/main.slint", config).unwrap();
}