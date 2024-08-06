use lopdf::Document;
use std::fs::File;
use std::io::{Read,Write};
use rfd::FileDialog;
use pdf_extract::extract_text;
use tempfile::NamedTempFile;


slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    ui.on_open_file(|| {
        // Open file dialog for user to select a PDF file
        if let Some(path) = FileDialog::new().add_filter("PDF files", &["pdf"]).pick_file() {
            // Open the selected PDF file
            let mut file = File::open(&path).expect("Cannot open file");
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).expect("Cannot read file");

            // Write the buffer to a temporary file
            let mut temp_file = NamedTempFile::new().expect("Cannot create temporary file");
            temp_file.write_all(&buffer).expect("Cannot write to temporary file");

            // Extract text using pdf-extract
            match extract_text(temp_file.path()) {
                Ok(text) => println!("{}", text),
                Err(e) => eprintln!("Failed to extract text: {}", e),
            }
            // ** TODO **
            //1.Retrieve the required data from the extracted text.
            //2.Add connection to db to save the data retrieved.
        } else {
            println!("No file selected.");
        }
    });

    ui.run()
}


