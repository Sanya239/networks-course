use eframe::egui;
use log::info;
use crate::control_flow::ControlFlow;
use anyhow::Result;
#[derive(Default)]
pub struct MyApp {
    state: AppState,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        // todo!()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.state.screen {
            Screen::Login => self.draw_login(ctx),
            Screen::Browser => self.draw_browser(ctx),
            Screen::FileView => self.draw_file(ctx),
        }
    }
}

#[derive(Default)]
enum Screen {
    #[default]
    Login,
    Browser,
    FileView,
}

#[derive(Default)]
struct AppState {
    screen: Screen,

    // login
    host: String,
    user: String,
    pass: String,

    // ftp
    ftp: Option<ControlFlow>,

    // browser
    current_path: String,
    files: Vec<String>,

    // file view
    file_content: Vec<u8>,

    // misc
    error: Option<String>,
    loading: bool,
}

impl MyApp {
    pub fn new() -> Self {
        Self {
            state: AppState {
                screen: Screen::Login,
                host: "10.41.193.143:2221".into(),
                user: "android".into(),
                pass: "android".into(),
                ftp: None,
                current_path: "/".into(),
                files: vec![],
                file_content: vec![],
                error: None,
                loading: false,
            },
        }
    }

    fn draw_login(&mut self, ctx: & eframe::egui::Context) {
        info!("draw login");
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("FTP Client");

            ui.label("Host");
            ui.text_edit_singleline(&mut self.state.host);

            ui.label("User");
            ui.text_edit_singleline(&mut self.state.user);

            ui.label("Password");
            ui.add(egui::TextEdit::singleline(&mut self.state.pass).password(true));

            if ui.button("Connect").clicked() {
                self.connect();
                if self.state.error.is_none() {
                    self.load_list();
                }
            }

            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

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
            ftp.user(&user);
            ftp.pass(&pass);
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

    fn draw_browser(&mut self, ctx: &egui::Context) {
        info!("draw browser");
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Path: {}", self.state.current_path));

            if ui.button("Refresh").clicked() {
                self.load_list();
            }

            if ui.button("Upload").clicked() {
                self.upload();
            }
            if ui.button("Quit").clicked() {
                // Send a command to the window/viewport to close
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for file in &self.state.files.clone() {
                    if ui.button(file).clicked() {
                        self.open_entry(file.clone());
                    }
                }
            });
        });
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
            if name.starts_with("type=dir") {
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
        info!("open file: {}", file);
        let ftp = self.state.ftp.as_mut().unwrap();
        info!("file is ready");
        match ftp.retr(&file) {
            Ok(data) => {
                self.state.file_content = data.clone();
                info!("{}", String::from_utf8_lossy(&*data).to_string());
                self.state.screen = Screen::FileView;
            }
            Err(e) => {
                self.state.error = Some(e.to_string());
            }
        }
    }

    fn draw_file(&mut self, ctx: &egui::Context) {
        info!("draw file");
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back").clicked() {
                self.state.screen = Screen::Browser;
            }
            if ui.button("Quit").clicked() {
                // Send a command to the window/viewport to close
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            let text = String::from_utf8_lossy(&self.state.file_content);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(text);
            });
        });
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