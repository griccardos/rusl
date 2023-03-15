fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        winres::WindowsResource::new().set_icon("src/icons/icon.ico").compile()?;
    }
    Ok(())
}
