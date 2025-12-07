//! Application entry point for the 2D SCA Tree viewer.
//!
//! This binary sets up eframe/egui and delegates all interactive
//! logic and rendering to [`Viewer`] from the `viewer` module.

mod viewer;

use viewer::Viewer;

/// Starts the native eframe application.
///
/// This function configures [`eframe::NativeOptions`] with default
/// settings and launches the main window titled `"2D SCA Tree"`.
/// All UI state and rendering are handled by [`Viewer`].
///
/// ### Returns
/// - `Ok(())` if the application runs to completion without errors.
/// - `Err` if eframe fails to create the native window or event loop.  
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "2D SCA Tree",
        options,
        Box::new(|_cc| {
            // Construct the root app state for the viewer.
            Ok(Box::new(Viewer::new()))
        }),
    )
}
