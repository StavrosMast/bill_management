# Personal Bills Tracker

## 📊 Overview

Personal Bills Tracker is a modern, user-friendly application designed to help you organize and manage your bills efficiently. Built with Rust and featuring a sleek UI powered by Slint, this app allows you to easily upload PDF bills, extract key information, and store it in a structured database for easy access and analysis.

## 🌟 Features

- 📁 PDF Upload: Easily upload your bills in PDF format.
- 🔍 Automatic Data Extraction: The app automatically extracts key information like invoice numbers, billing periods, and due dates.
- 💾 Database Storage: All extracted information is stored in a SQLite database for easy retrieval and management.
- 📊 Data Visualization: View your bill information in a clean, easy-to-read table format.
- 🔄 Real-time Updates: The UI updates in real-time as you add new bills.
- 🎨 Modern UI: A sleek, intuitive interface built with Slint.

## 🚀 Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo
- SQLite

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/StavrosMast/personal-bills-tracker.git
   cd personal-bills-tracker
   ```

2. Set up the environment:
   - Create a `.env` file in the project root.
   - Add the following lines to the `.env` file:
     ```
     DATABASE_URL=path/to/your/database.db
     INVOICES_STMT=your_insert_statement_here
     ```

3. Build the project:
   ```
   cargo build
   ```

4. Run the application:
   ```
   cargo run
   ```

## 🛠 Usage

1. Launch the application.
2. Use the "Upload PDF" button to select and upload a bill.
3. The app will automatically extract and display the bill information.
4. Use the table view to see all your stored bills.
5. (Optional) Use the ComboBox to filter bills by issuer.

## 📝 License

This project is [MIT](https://choosealicense.com/licenses/mit/) licensed.

## 🙏 Acknowledgements

- [Rust](https://www.rust-lang.org/)
- [Slint](https://slint-ui.com/)
- [SQLite](https://www.sqlite.org/)
- [pdf-extract](https://crates.io/crates/pdf-extract)
