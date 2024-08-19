use std::fs::File;
use std::io::{Read,Write};
use rfd::FileDialog;
use pdf_extract::extract_text;
use tempfile::NamedTempFile;
use regex::Regex;
use std::env;
use dotenv::dotenv;
use rusqlite::{Connection,Result};
use slint::{Model,PlatformError,ModelRc,SharedString,VecModel,StandardListViewItem,ModelTracker};
use rusqlite::Error as RusqliteError;
use std::fmt;
use std::rc::Rc;
slint::include_modules!();

struct CustomModel {
    inner: ModelRc<StandardListViewItem>,
}

impl Model for CustomModel {
    type Data = ModelRc<StandardListViewItem>;

    fn row_count(&self) -> usize {
        self.inner.row_count()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        Some(ModelRc::new(self.inner.row_data(row)?))
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        self.inner.model_tracker()
    }
}

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
        ModelRc::new(VecModel::from(vec![])) // Return an empty model if there's an error
    });

    // Pass the data to the UI
    ui.global::<TableViewPageAdapter>().set_row_data(table_data);
    // ui.set_table_data(table_data);
// Convert data into the format expected by the Slint UI
// let table_data: Vec<Vec<String>> = invoices
//     .into_iter()
//     .map(|row| vec![row.invoice_number, row.start_date, row.end_date, row.date_due])
//     .collect();
// println!("table data:{}",table_data);
// // Pass data to the UI
// ui.set_table_data(table_data);
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

// fn fetch_invoices_from_db() -> Result<ModelRc<ModelRc<StandardListViewItem>>> {
//     dotenv::dotenv().ok(); // Load the .env file
//     let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//     let conn = Connection::open(database_url)?;

//     let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;
//     // let invoice_iter = stmt.query_map([], |row| {
//     //     Ok(vec![
//     //         row.get::<_, String>(0)?.into(), // Convert to SharedString
//     //         row.get::<_, String>(1)?.into(), // Convert to SharedString
//     //         row.get::<_, String>(2)?.into(), // Convert to SharedString
//     //         row.get::<_, String>(3)?.into(), // Convert to SharedString
//     //     ])
//     // })?;
//     let invoice_iter = stmt.query_map([], |row| {
//         let invoice_number: SharedString = row.get::<_, String>(0)?.into();
//         let period_start_date: SharedString = row.get::<_, String>(1)?.into();
//         let period_end_date: SharedString = row.get::<_, String>(2)?.into();
//         let date_due: SharedString = row.get::<_, String>(3)?.into();

//         let item_text = format!(
//             "{} | {} | {} | {}",
//             invoice_number, period_start_date, period_end_date, date_due
//         );

//         Ok(StandardListViewItem::from(item_text.into())) // Convert to StandardListViewItem
//     })?;

//     // let mut invoices = Vec::new();
//     // for invoice in invoice_iter {
//     //     let row_data: Vec<StandardListViewItem> = invoice?;
//     //     let row_model = VecModel::from(row_data); // Convert Vec<SharedString> to VecModel<SharedString>
//     //     invoices.push(ModelRc::new(row_model)); // Convert to ModelRc<VecModel<SharedString>>
//     // }
//     let mut invoices = Vec::new(); // Create a vector to hold all the StandardListViewItems

//     for invoice in invoice_iter {
//         invoices.push(ModelRc::new(invoice?)); // Push each StandardListViewItem into the invoices vector
//     }
//     let vec_model = VecModel::from(invoices); // Create a VecModel from the vector of ModelRc<StandardListViewItem>
//     Ok(ModelRc::new(ModelRc::new(vec_model)))
//     // Ok(ModelRc::new(VecModel::from(invoices))) // Convert Vec<ModelRc<VecModel<SharedString>>> to ModelRc<VecModel<ModelRc<VecModel<SharedString>>>>
// }
fn fetch_invoices_from_db() -> Result<ModelRc<ModelRc<StandardListViewItem>>, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok(); // Load the .env file
    let database_url = env::var("DATABASE_URL")?;
    let conn = Connection::open(database_url)?;

    let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;

    let invoice_iter = stmt.query_map([], |row| {
        let invoice_number: SharedString = row.get::<_, String>(0)?.into();
        let period_start_date: SharedString = row.get::<_, String>(1)?.into();
        let period_end_date: SharedString = row.get::<_, String>(2)?.into();
        let date_due: SharedString = row.get::<_, String>(3)?.into();

        let item_text = format!(
            "{} | {} | {} | {}",
            invoice_number, period_start_date, period_end_date, date_due
        );

        Ok(StandardListViewItem::from(item_text.into()))
    })?;

    // let mut invoices = Vec::new();

    // for invoice in invoice_iter {
    //     invoices.push(ModelRc::new(invoice?)); // Collect StandardListViewItems
    // }

    // // Create a VecModel<StandardListViewItem>
    // let vec_model = VecModel::from(invoices);

    // // Wrap the VecModel in a ModelRc
    // let wrapped_vec_model = ModelRc::new(vec_model);

    // // Wrap the ModelRc<VecModel<StandardListViewItem>> in another ModelRc
    // let final_model = ModelRc::new(wrapped_vec_model);

    // Ok(final_model)
    let mut invoices = Vec::new();

    for invoice in invoice_iter {
        let item = invoice?; // Get the StandardListViewItem
        invoices.push(item); // Collect StandardListViewItems
    }

    // // Create a VecModel<StandardListViewItem>
    // let vec_model = VecModel::from(invoices);

    // // Wrap the VecModel in a ModelRc
    // let wrapped_vec_model = ModelRc::new(vec_model);

    // // Wrap the ModelRc<VecModel<StandardListViewItem>> in another ModelRc
    // let final_model = ModelRc::new(wrapped_vec_model);
    let mut invoices = Vec::new();

for invoice in invoice_iter {
    let item = invoice?; // Get the StandardListViewItem
    invoices.push(item); // Collect StandardListViewItems
}

// Create a VecModel<StandardListViewItem>
let vec_model = VecModel::from(invoices);

// Wrap the VecModel in a ModelRc
let wrapped_vec_model = ModelRc::new(vec_model);

// Create a CustomModel that wraps the ModelRc
let custom_model = CustomModel { inner: wrapped_vec_model };

// Wrap the CustomModel in a ModelRc
let final_model = ModelRc::new(custom_model);

Ok(final_model)
}


// fn fetch_invoices_from_db() -> Result<ModelRc<VecModel<StandardListViewItem>>> {
//     dotenv::dotenv().ok(); // Load the .env file
//     let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//     let conn = Connection::open(database_url)?;

//     let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;
//     let invoice_iter = stmt.query_map([], |row| {
//         Ok(StandardListViewItem {
//             text: slint::format!(
//                 "{} | {} | {} | {}",
//                 row.get::<_, String>(0)?,
//                 row.get::<_, String>(1)?,
//                 row.get::<_, String>(2)?,
//                 row.get::<_, String>(3)?
//             ).into(),
//         })
//     })?;

//     let invoices: Vec<StandardListViewItem> = invoice_iter
//         .filter_map(|result| result.ok())
//         .collect();

//     Ok(ModelRc::new(VecModel::from(invoices)))
// }


// fn fetch_invoices_from_db() -> Result<ModelRc<VecModel<StandardListViewItem>>, Box<dyn std::error::Error>> {
//     dotenv::dotenv().ok(); // Load the .env file
//     let database_url = env::var("DATABASE_URL")?;
//     let conn = rusqlite::Connection::open(database_url)?;

//     let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;
//     let invoice_iter = stmt.query_map([], |row| {
//         let item = StandardListViewItem::default(); // Create a default instance
//         let text = format!(
//             "{} | {} | {} | {}",
//             row.get::<_, String>(0)?,
//             row.get::<_, String>(1)?,
//             row.get::<_, String>(2)?,
//             row.get::<_, String>(3)?
//         );
//         item.set_text(text.into()); // Use the setter method to set the text
//         Ok(item)
//     })?;

//     let invoices: Vec<StandardListViewItem> = invoice_iter
//         .filter_map(|result| result.ok())
//         .collect();

//     Ok(ModelRc::new(VecModel::from(invoices)))
// }


// fn fetch_invoices_from_db() -> Result<ModelRc<VecModel<VecModel<SharedString>>>, Box<dyn std::error::Error>> {
//     dotenv::dotenv().ok(); // Load the .env file
//     let database_url = env::var("DATABASE_URL")?;
//     let conn = Connection::open(database_url)?;

//     let mut stmt = conn.prepare("SELECT invoice_number, period_start_date, period_end_date, date_due FROM invoices")?;
//     let invoice_iter = stmt.query_map([], |row| {
//         let row_data = vec![
//             row.get::<_, String>(0)?.into(), // Convert to SharedString
//             row.get::<_, String>(1)?.into(), // Convert to SharedString
//             row.get::<_, String>(2)?.into(), // Convert to SharedString
//             row.get::<_, String>(3)?.into(), // Convert to SharedString
//         ];
//         Ok(VecModel::from(row_data))
//     })?;

//     let invoices: Vec<VecModel<SharedString>> = invoice_iter
//         .filter_map(|result| result.ok())
//         .collect();

//     Ok(ModelRc::new(VecModel::from(invoices)))
// }
