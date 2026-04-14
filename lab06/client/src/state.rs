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

    // login
    pub(crate) host: String,
    pub(crate) user: String,
    pub(crate) pass: String,

    // ftp
    pub(crate) ftp: Option<ControlFlow>,

    // browser
    pub(crate) current_path: String,
    pub(crate) files: Vec<String>,

    // file view
    pub(crate) file_content: Vec<u8>,

    // misc
    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
}


impl Default for AppState {
    fn default() -> Self {
        Self {
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
        }
    }
}