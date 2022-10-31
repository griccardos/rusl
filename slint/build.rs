fn main() {
    slint_build::compile_with_config(
        "src/mainwindow.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent-dark".into()),
    )
    .unwrap();
}
