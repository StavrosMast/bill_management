use std::fs::File;
use std::io::{Read,Write};
use rfd::FileDialog;
use pdf_extract::extract_text;
use tempfile::NamedTempFile;
use regex::Regex;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    // function for the callback of ComboBox
    ui.on_selection_changed(|new_index,new_value| {
        println!("Selected index: {}", new_index);
        println!("Selected value: {}", new_value);
    });

    // callback for the button to open and read the pdf file
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
                Ok(text) => {
                    println!("Extracted Text:\n{}", text);
                    // Define regex patterns for dates and prices
                    let date_re = Regex::new(r"\b\d{4}-\d{2}-\d{2}\b").expect("Invalid regex for dates");
                    let price_re = Regex::new(r"\b\d+\.\d{2}\b").expect("Invalid regex for prices");
                    // Define regex pattern for the invoice number
                    let invoice_re = Regex::new(r"Invoice number/Αριθμός τιμολογίου: (\d+\[\d+\]|\d+)").expect("Invalid regex for invoice number");

                    // Find and print the invoice number
                    if let Some(captures) = invoice_re.captures(&text) {
                        if let Some(invoice_number) = captures.get(1) {
                            println!("Found invoice number: {}", invoice_number.as_str());
                        }
                    }
                    // Find and print all dates
                    for date in date_re.find_iter(&text) {
                        println!("Found date: {}", date.as_str());
                    }

                    // Find and print all prices
                    for price in price_re.find_iter(&text) {
                        println!("Found price: {}", price.as_str());
                    }
                },
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


