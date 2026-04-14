use crate::control_flow::ControlFlow;
use crate::state::{AppState, Screen};
use anyhow::Result;
use eframe::egui;
use log::info;

#[derive(Default)]
pub struct MyApp {
    state: AppState,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        // todo!()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.login_section(ui);
            match self.state.screen {
                Screen::Browser => self.draw_browser(ui),
                Screen::FileView => self.draw_file(ui),
                _ => {}
            }

            self.quit_button(ctx, ui);
        });
    }
}

impl MyApp {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }

    fn quit_button(&self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if ui.button("Quit").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn login_section(&mut self, ui: &mut egui::Ui) {
        ui.set_width(ui.available_width());
        ui.group(|ui| {
            egui::Grid::new("signin").num_columns(2)
                .spacing([20.0, 8.0]).show(ui, |ui| {
                ui.label("Host");
                ui.add(egui::TextEdit::singleline(&mut self.state.host).password(false).desired_width(200.0));
                ui.end_row();

                ui.label("User");
                ui.add(egui::TextEdit::singleline(&mut self.state.user).password(false).desired_width(200.0));
                ui.end_row();

                ui.label("Password");
                ui.add(egui::TextEdit::singleline(&mut self.state.pass).password(true).desired_width(200.0));

                if ui.button("Connect").clicked() {
                    self.connect();
                    if self.state.error.is_none() {
                        self.load_list();
                    }
                }
                ui.end_row();
            });
            if let Some(err) = &self.state.error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }

    fn connect(&mut self) {
        info!("connect to server");
        let host = self.state.host.clone();
        let user = self.state.user.clone();
        let pass = self.state.pass.clone();

        self.state.loading = true;

        let result = || -> Result<ControlFlow> {
            let mut ftp = ControlFlow::connect(&host)?;
            ftp.user(&user)?;
            ftp.pass(&pass)?;
            ftp.type_i()?;
            Ok(ftp)
        }();

        match result {
            Ok(ftp) => {
                self.state.ftp = Some(ftp);
                self.state.screen = Screen::Browser;
                self.state.error = None;
            }
            Err(e) => {
                self.state.error = Some(e.to_string());
            }
        }

        self.state.loading = false;
    }

    fn draw_browser(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("Path: {}", self.state.current_path));

        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.load_list();
            }

            if ui.button("Upload").clicked() {
                self.upload();
            }
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            for file in self.state.files.clone() {
                ui.horizontal(|ui| {
                    let fields = file.split(";").collect::<Vec<_>>();
                    let mut filename = file.clone();
                    if fields.len() > 1 {
                        filename = fields.last().unwrap().trim().to_string();
                    }
                    ui.label(filename);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Delete").clicked() {
                            self.delete(file.clone());
                        }

                        if ui.button("Open").clicked() {
                            self.open_entry(file.clone());
                        }
                    });
                });

                ui.separator();
            }
        });
    }
    fn delete(&mut self, file: String) {
        let ftp = self.state.ftp.as_mut().unwrap();

        if let Err(e) = ftp.dele(&file) {
            self.state.error = Some(e.to_string());
            return;
        }

        self.load_list();
    }


    fn load_list(&mut self) {
        info!("load list");
        let ftp = self.state.ftp.as_mut().unwrap();

        let result = ftp.list();

        match result {
            Ok(data) => {
                let text = String::from_utf8_lossy(&data);
                self.state.files = text.lines().map(|s| s.to_string()).collect();
            }
            Err(e) => {
                self.state.error = Some(e.to_string());
            }
        }
        info!("loaded list: {:?}", self.state.files.join("\n"));
    }

    fn open_entry(&mut self, name: String) {
        let fields = name.split(";").collect::<Vec<_>>();
        if fields.len() > 1 {
            let filename = fields.last().unwrap().trim().to_string();
            if name.to_lowercase().find("type=dir").is_some() {
                self.cwd(filename);
            } else {
                self.open_file(filename);
            }
        } else {
            self.cwd(name.clone());
            if self.state.error.is_some() {
                self.state.error = None;
                self.open_file(name);
            }
        }
    }

    fn cwd(&mut self, dir: String) {
        info!("cwd: {}", dir);
        let ftp = self.state.ftp.as_mut().unwrap();

        if let Err(e) = ftp.cwd(&dir) {
            self.state.error = Some(e.to_string());
            return;
        }

        self.state.current_path = dir;

        if let Ok(data) = ftp.list() {
            let text = String::from_utf8_lossy(&data);
            self.state.files = text.lines().map(|s| s.to_string()).collect();
        }
    }

    fn open_file(&mut self, file: String) {
        let ftp = self.state.ftp.as_mut().unwrap();

        match ftp.retr(&file) {
            Ok(data) => {
                self.state.file_content = data;
                self.state.editing_file = Some(file);
                self.state.screen = Screen::FileView;
            }
            Err(e) => {
                self.state.error = Some(e.to_string());
            }
        }
    }

    fn draw_file(&mut self, ui: &mut egui::Ui) {
        let filename = self.state.editing_file.clone().unwrap_or_default();

        ui.heading(format!("Editing: {}", filename));

        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.state.screen = Screen::Browser;
                self.state.editing_file = None;
            }

            if ui.button("Commit").clicked() {
                self.commit();
            }

            if ui.button("Save to disk").clicked() {
                self.save_to_disk();
            }
        });

        let mut text = String::from_utf8_lossy(&self.state.file_content).to_string();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(25),
            );
        });

        self.state.file_content = text.into_bytes();
    }

    fn commit(&mut self) {
        let ftp = self.state.ftp.as_mut().unwrap();

        if let Some(file) = &self.state.editing_file {
            if let Err(e) = ftp.stor(file, &self.state.file_content) {
                self.state.error = Some(e.to_string());
                return;
            }
        }

        self.state.screen = Screen::Browser;
        self.load_list();
    }

    fn save_to_disk(&mut self) {
        let default_name = self
            .state
            .editing_file
            .clone()
            .unwrap_or_else(|| "file.txt".to_string());

        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .save_file()
        {
            if let Err(e) = std::fs::write(&path, &self.state.file_content) {
                self.state.error = Some(e.to_string());
            }
        }
    }

    fn upload(&mut self) {
        info!("upload");
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let data = std::fs::read(&path).unwrap();
            let filename = path.file_name().unwrap().to_string_lossy().to_string();

            let ftp = self.state.ftp.as_mut().unwrap();

            if let Err(e) = ftp.stor(&filename, &data) {
                self.state.error = Some(e.to_string());
            }
        }
    }
}
