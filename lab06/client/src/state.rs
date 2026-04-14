use crate::control_flow::ControlFlow;

#[derive(Default)]
pub enum Screen {
    #[default]
    Login,
    Browser,
    FileView,
}

pub struct AppState {
    pub(crate) screen: Screen,

    pub(crate) host: String,
    pub(crate) user: String,
    pub(crate) pass: String,

    pub(crate) ftp: Option<ControlFlow>,

    pub(crate) current_path: String,
    pub(crate) files: Vec<String>,

    pub(crate) editing_file: Option<String>,
    pub(crate) file_content: Vec<u8>,

    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
}


impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::Login,
            host: "0.0.0.0:2121".into(),
            user: "dlpuser".into(),
            pass: "rNrKYTX9g7z3RgJRmxWuGHbeu".into(),
            ftp: None,
            current_path: "/".into(),
            files: vec![],
            editing_file: None,
            file_content: vec![],
            error: None,
            loading: false,
        }
    }
}