use eframe::egui;
use egui::{Color32, Pos2, Stroke};

use serde::{Deserialize, Serialize};

use std::io::{BufRead, BufReader, Write};

use std::net::{TcpListener, TcpStream};

use std::sync::{Arc, Mutex};

use std::thread;

use std::time::{Duration, Instant};

use anyhow::Result;
#[derive(Serialize, Deserialize, Clone)]
struct Point {
    x: f32,
    y: f32,
}

enum Mode {
    Menu,
    Host,
    Client,
}

struct PaintApp {
    mode: Mode,

    address_input: String,

    points: Arc<Mutex<Vec<Point>>>,

    clients: Arc<Mutex<Vec<TcpStream>>>,

    last_send: Instant,
}

impl Default for PaintApp {
    fn default() -> Self {
        Self {
            mode: Mode::Menu,

            address_input: "127.0.0.1:5000".to_string(),

            points: Arc::new(Mutex::new(Vec::new())),

            clients: Arc::new(Mutex::new(Vec::new())),

            last_send: Instant::now(),
        }
    }
}

impl PaintApp {
    fn start_host(&mut self) -> Result<()> {
        self.mode = Mode::Host;

        let addr = self.address_input.clone();

        let clients = self.clients.clone();

        thread::spawn(move || -> Result<()> {
            let listener = TcpListener::bind(addr.clone())?;

            println!("Hosting on {addr}");

            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    println!("Client connected");

                    clients.lock().unwrap().push(stream);
                }
            }
            return Ok(());
        });
        return Ok(());
    }

    fn start_client(&mut self) -> Result<()> {
        self.mode = Mode::Client;

        let addr = self.address_input.clone();

        let points = self.points.clone();

        thread::spawn(move || -> Result<()> {
            let stream = TcpStream::connect(addr)?;

            let reader = BufReader::new(stream);

            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(point) = serde_json::from_str::<Point>(&line) {
                        points.lock().unwrap().push(point);
                    }
                }
            }
            return Ok(());
        });
        return Ok(());
    }

    fn send_point(&mut self, point: Point) -> Result<()> {
        if self.last_send.elapsed() < Duration::from_millis(100) {
            return Ok(());
        }

        self.last_send = Instant::now();

        let json = serde_json::to_string(&point)? + "\n";

        let mut clients = self.clients.lock().unwrap();

        clients.retain_mut(|stream| stream.write_all(json.as_bytes()).is_ok());
        return Ok(());
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui) -> Result<()> {
        let available = ui.available_size();

        let (response, painter) = ui.allocate_painter(available, egui::Sense::drag());

        if matches!(self.mode, Mode::Host) {
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let point = Point { x: pos.x, y: pos.y };

                    self.points.lock().unwrap().push(point.clone());

                    self.send_point(point);
                }
            }
        }

        let points = self.points.lock().unwrap();

        for window in points.windows(2) {
            let a = Pos2::new(window[0].x, window[0].y);

            let b = Pos2::new(window[1].x, window[1].y);

            painter.line_segment([a, b], Stroke::new(2.0, Color32::WHITE));
        }
        return Ok(());
    }
}

impl eframe::App for PaintApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| match self.mode {
            Mode::Menu => {
                ui.heading("Network Paint");

                ui.horizontal(|ui| {
                    if ui.button("Create sesion").clicked() {
                        self.start_host();
                    }

                    if ui.button("Join session").clicked() {
                        self.start_client();
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Address:");

                    ui.text_edit_singleline(&mut self.address_input);
                });
            }

            Mode::Host => {
                ui.label(format!("HOST MODE — clients can connect to {0}", self.address_input));

                self.draw_canvas(ui);
            }

            Mode::Client => {
                ui.label("CLIENT MODE");

                self.draw_canvas(ui);
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Network Canvas",
        options,
        Box::new(|_| Box::new(PaintApp::default())),
    )
}
