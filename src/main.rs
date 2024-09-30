use std::fs::File;
use std::io::{Read,Write};
use rfd::FileDialog;
use pdf_extract::extract_text;
use tempfile::NamedTempFile;
use regex::Regex;
use std::env;
use dotenv::dotenv;
use rusqlite::{Connection,Result};
use slint::{PlatformError, ModelRc, SharedString, VecModel, StandardListViewItem};
use rusqlite::Error as RusqliteError;
use std::fmt;
use std::thread;
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

    // Retrieve data from the database
    let table_data = fetch_invoices_from_db().unwrap_or_else(|err| {
        eprintln!("Failed to fetch invoices: {}", err);
        vec![]
    });

    let model_data = ModelRc::new(VecModel::from(
        table_data.into_iter().map(|row| 
            ModelRc::new(VecModel::from(
                row.into_iter().map(|item| StandardListViewItem::from(SharedString::from(item))).collect::<Vec<_>>()
            ))
        ).collect::<Vec<_>>()
    ));

    ui.global::<TableViewPageAdapter>().set_row_data(model_data);
    
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
                    let result = get_data_for_epic(&text);
                    match result {
                        Ok(_) => println!("Operation was successful."),
                        Err(e) => println!("Operation failed with error: {:?}", e),
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

fn get_data_for_epic(text: &str) -> Result<(), AppError>{

    dotenv().ok(); // Load the .env file

    // Read the value from the .env file
    let database_url = env::var("DATABASE_URL").expect("You've not set the DATABASE_URL");

    let conn = Connection::open(database_url)?;

    // Define regex patterns
    let invoice_re = Regex::new(r"Invoice number/Αριθμός τιμολογίου: (\d+\[\d+\]|\d+)").expect("Invalid regex for invoice number");
    let period_re = Regex::new(r"Για την περίοδο (\d{2} \w+ \d{4}) - (\d{2} \w+ \d{4})").expect("Invalid regex for period");
    let date_due_re = Regex::new(r"Πληρωτέο μέχρι (\d{2}/\d{2}/\d{4})").expect("Invalid regex for date due");

    let mut date_due = None;
    let mut start_date = None;
    let mut end_date = None;
    let mut invoice_number = None;
    // Find and print the date due
    if let Some(captures) = date_due_re.captures(&text) {
        if let Some(date_due_capture) = captures.get(1) {
            date_due = Some(date_due_capture.as_str().to_string());
            println!("Found Date due: {}", date_due.as_ref().unwrap());
            // Insert the date due into the database
        }
    }else {
        println!("No Date due found.")
    }
    // Find and extract the start and end dates
    if let Some(captures) = period_re.captures(&text) {
        if let (Some(start_date_capture), Some(end_date_capture)) = (captures.get(1), captures.get(2)) {
            start_date = Some(start_date_capture.as_str().to_string());
            end_date = Some(end_date_capture.as_str().to_string());
            println!("Start date: {}", start_date.as_ref().unwrap());
            println!("End date: {}", end_date.as_ref().unwrap());
        }
    } else {
        println!("No period found.");
    }
    // Find and print the invoice number
    if let Some(captures) = invoice_re.captures(&text) {
        if let Some(invoice_number_capture) = captures.get(1) {
            invoice_number = Some(invoice_number_capture.as_str().to_string());
            println!("Found invoice number: {}", invoice_number.as_ref().unwrap());
        }
    }
    // Insert the values into the database
    if let (Some(invoice_number), Some(start_date), Some(end_date), Some(date_due)) = (invoice_number, start_date, end_date, date_due) {
        let insert_to_invoices_stmt = env::var("INVOICES_STMT").expect("You've not set the INVOICES_STMT");
        if let Err(e) = conn.execute(
            insert_to_invoices_stmt.as_str(),
            &[&invoice_number, &start_date, &end_date, &date_due],
        ) {
            eprintln!("Failed to insert invoice data: {}", e);
        } else {
            println!("Successfully inserted invoice data.");
        }
    } else {
        println!("Incomplete data, nothing to insert.");
    }
    Ok(())
}

fn fetch_invoices_from_db() -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let conn = Connection::open(database_url)?;

    let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;

    let invoice_iter = stmt.query_map([], |row| {
        Ok(vec![
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
        ])
    })?;

    let invoices: Vec<Vec<String>> = invoice_iter.collect::<Result<_, _>>()?;
    Ok(invoices)
}


fn place_searched_users(ui: AppWindow) {
    let ui_thread = ui.as_weak();
    thread::spawn(move || { 
        let ui = ui_thread.clone();
        let do_it = slint::invoke_from_event_loop(move || {
            let ui = ui.unwrap();
            let table_data = fetch_invoices_from_db().unwrap_or_else(|err| {
                eprintln!("Failed to fetch invoices: {}", err);
                vec![]
            });

            let model_data = ModelRc::new(VecModel::from(
                table_data.into_iter().map(|row| 
                    ModelRc::new(VecModel::from(
                        row.into_iter().map(|item| StandardListViewItem::from(SharedString::from(item))).collect::<Vec<_>>()
                    ))
                ).collect::<Vec<_>>()
            ));

            ui.global::<TableViewPageAdapter>().set_row_data(model_data);
        });

        if let Err(e) = do_it {
            eprintln!("Error: {:?}", e);
        }
    });
}