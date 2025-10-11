use markdownify::docx;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::panic::catch_unwind;
use std::path::Path;
pub trait ExtendedTrait {
    fn name(&self) -> String;
    ///lowercase extensions
    fn extensions(&self) -> Vec<String>;
    fn to_string(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>>;
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum ExtendedType {
    Pdf,
    Office,
}

impl ExtendedTrait for ExtendedType {
    fn extensions(&self) -> Vec<String> {
        match self {
            ExtendedType::Pdf => vec!["pdf".to_string()],
            ExtendedType::Office => vec![
                "docx".to_string(),
                "xlsx".to_string(),
                "ods".to_string(),
                "xls".to_string(),
                "xlsm".to_string(),
                "xlsb".to_string(),
                "pptx".to_string(),
                "odt".to_string(),
                "odp".to_string(),
            ],
        }
    }

    fn to_string(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            ExtendedType::Pdf => Ok(extract_pdf(path)?),
            ExtendedType::Office => Ok(extract_office(path)?),
        }
    }

    fn name(&self) -> String {
        match self {
            ExtendedType::Pdf => "Pdf",
            ExtendedType::Office => "Office",
        }
        .to_string()
    }
}
impl From<&str> for ExtendedType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "pdf" => ExtendedType::Pdf,
            "office" => ExtendedType::Office,
            _ => panic!("unknown extended type"),
        }
    }
}

fn extract_pdf(path: &Path) -> Result<String, Box<dyn Error>> {
    let path = path.to_owned();
    //because the library panics, we need to catch panics
    let res = catch_unwind(|| pdf_extract::extract_text(&path));
    Ok(res.map_err(|_| "Panicked".to_string())??)
}

fn extract_office(path: &Path) -> Result<String, Box<dyn Error>> {
    let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();
    let string = match ext.as_str() {
        "docx" => docx::docx_convert(&path)?,
        "xlsx" | "ods" | "xls" | "xlsm" | "xlsb" => markdownify::sheets::sheets_convert(&path)?,
        "pptx" => markdownify::pptx::pptx_converter(&path)?,
        "odt" | "odp" => markdownify::opendoc::opendoc_convert(&path)?,
        _ => return Err("unknown extension".into()),
    };
    Ok(string)
}
