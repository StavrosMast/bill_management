slint::include_modules!();
use lopdf::Document;
use std::fs::File;
use std::io::Read;
use rfd::FileDialog;
fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    // ui.on_request_increase_value({
    //     let ui_handle = ui.as_weak();
    //     move || {
    //         let ui = ui_handle.unwrap();
    //         ui.set_counter(ui.get_counter() + 1);
    //     }
    // });

    ui.run()
}
