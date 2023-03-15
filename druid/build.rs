//This adds icon for windows.
//For OSX use `cargo bundle`
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        winres::WindowsResource::new().set_icon("src/icons/icon.ico").compile()?;
    }
    Ok(())
}
