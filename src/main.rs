use std::fs::File;
use std::io::{Read,Write};
use rfd::FileDialog;
use pdf_extract::extract_text;
use tempfile::NamedTempFile;
use regex::Regex;
use std::env;
use dotenv::dotenv;
use rusqlite::{Connection,Result};
use slint::PlatformError;
use rusqlite::Error as RusqliteError;
use std::fmt;

slint::include_modules!();


#[derive(Debug)]
enum AppError {
    Platform(PlatformError),
    Sqlite(RusqliteError),
    // Add more variants if needed
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Platform(err) => write!(f, "Platform error: {}", err),
            AppError::Sqlite(err) => write!(f, "SQLite error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

impl From<PlatformError> for AppError {
    fn from(err: PlatformError) -> Self {
        AppError::Platform(err)
    }
}

impl From<RusqliteError> for AppError {
    fn from(err: RusqliteError) -> Self {
        AppError::Sqlite(err)
    }
}

fn main() -> Result<(), AppError> {
    let ui = AppWindow::new()?;

    dotenv().ok(); // Load the .env file

    // Read the value from the .env file
    let database_url = env::var("DATABASE_URL").expect("You've not set the DATABASE_URL");

    let conn = Connection::open(database_url)?;

    // function for the callback of ComboBox
    ui.on_selection_changed(|new_index,new_value| {
        println!("Selected index: {}", new_index);
        println!("Selected value: {}", new_value);
    });

    // callback for the button to open and read the pdf file
    ui.on_open_file(move|| {
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

                    // Define regex patterns
                    let invoice_re = Regex::new(r"Invoice number/Αριθμός τιμολογίου: (\d+\[\d+\]|\d+)").expect("Invalid regex for invoice number");
                    let period_re = Regex::new(r"Για την περίοδο (\d{2} \w+ \d{4}) - (\d{2} \w+ \d{4})").expect("Invalid regex for period");
                    let date_due_re = Regex::new(r"Πληρωτέο μέχρι (\d{2}/\d{2}/\d{4})").expect("Invalid regex for date due");
                    
                    // Find and print the invoice number
                    if let Some(captures) = date_due_re.captures(&text) {
                        if let Some(date_due) = captures.get(1) {
                            let date_due = date_due.as_str();
                            println!("Found Date due: {}", date_due);
                            // Insert the invoice number into the database
                        }
                    }else {
                        println!("No Date due found.")
                    }
                    // Find and extract the start and end dates
                    if let Some(captures) = period_re.captures(&text) {
                        if let (Some(start_date), Some(end_date)) = (captures.get(1), captures.get(2)) {
                            let start_date = start_date.as_str();
                            let end_date = end_date.as_str();
                            println!("Start date: {}", start_date);
                            println!("End date: {}", end_date);
                        }
                    } else {
                        println!("No period found.");
                    }
                    // Find and print the invoice number
                    if let Some(captures) = invoice_re.captures(&text) {
                        if let Some(invoice_number) = captures.get(1) {
                            let invoice_number = invoice_number.as_str();
                            println!("Found invoice number: {}", invoice_number);
                            // Insert the invoice number into the database
                            let insert_to_invoices_stmt = env::var("INVOICES_STMT").expect("You've not set the INVOICES_TABLE");
                            if let Err(e) = conn.execute(
                                insert_to_invoices_stmt.as_str(),
                                &[&invoice_number],
                            ) {
                                eprintln!("Failed to insert invoice number: {}", e);
                            }
                        }
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

    ui.run().map_err(AppError::from)?;
    Ok(())
}


