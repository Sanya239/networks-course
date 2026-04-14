use crate::control_flow::ControlFlow;
use anyhow::Result;
use crate::app::MyApp;

// Import necessary parts of eframe and egui
mod control_flow;
mod app;
mod state;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "egui Demo",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

