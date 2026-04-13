use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::path::{Path, PathBuf};
use std::thread;
use log::info;

fn main() -> std::io::Result<()> {
    env_logger::init();
    let listener = TcpListener::bind("0.0.0.0:2121")?;
    info!("FTP server listening on 2121");

    for stream in listener.incoming() {
        let stream = stream?;
        thread::spawn(|| handle_client(stream));
    }

    Ok(())
}

struct Session {
    cwd: PathBuf,
    data_addr: Option<SocketAddr>,
    pasv_listener: Option<TcpListener>,
    logged_in: bool,
    pending_user: Option<String>,
}

fn open_data_connection(session: &mut Session) -> Option<TcpStream> {
    if let Some(listener) = session.pasv_listener.take() {
        if let Ok((stream, _)) = listener.accept() {
            return Some(stream);
        }
    }

    if let Some(addr) = session.data_addr.take() {
        if let Ok(stream) = TcpStream::connect(addr) {
            return Some(stream);
        }
    }

    None
}
use std::time::{SystemTime, UNIX_EPOCH};

fn format_ftp_time(time: SystemTime) -> Result<String, ()> {
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    Ok(datetime.format("%Y%m%d%H%M%S").to_string())
}

fn handle_client(mut stream: TcpStream) {
    info!("New connection from: {}", stream.peer_addr().unwrap());
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    let mut session = Session {
        cwd: std::env::current_dir().unwrap(),
        data_addr: None,
        pasv_listener: None,
        logged_in: false,
        pending_user: None,
    };

    writeln!(stream, "220 Simple FTP Server").ok();

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }

        let line = line.trim_end();
        let mut parts = line.split_whitespace();

        let cmd = parts.next().unwrap_or("").to_uppercase();
        let arg = parts.next();
        info!("{}", cmd);

        if !session.logged_in
            && cmd != "USER"
            && cmd != "PASS"
            && cmd != "QUIT"
        {
            writeln!(stream, "530 Please login with USER and PASS").ok();
            continue;
        }
        match cmd.as_str() {
            "USER" => {
                if let Some(user) = arg {
                    if user.len() >= 3 {
                        session.pending_user = Some(user.to_string());
                        writeln!(stream, "331 Username OK, need password").ok();
                    } else {
                        writeln!(stream, "530 Invalid username").ok();
                    }
                } else {
                    writeln!(stream, "501 Missing username").ok();
                }
            }

            "PASS" => {
                if let Some(pass) = arg {
                    if session.pending_user.is_some() && pass.len() >= 3 {
                        session.logged_in = true;
                        writeln!(stream, "230 Login successful").ok();
                    } else {
                        writeln!(stream, "530 Login incorrect").ok();
                    }
                } else {
                    writeln!(stream, "501 Missing password").ok();
                }
            }

            "PASV" => {
                session.pasv_listener = None;

                let listener = TcpListener::bind("0.0.0.0:0");

                match listener {
                    Ok(listener) => {
                        let addr = listener.local_addr().unwrap();

                        let ip = match stream.local_addr() {
                            Ok(a) => a.ip(),
                            Err(_) => {
                                writeln!(stream, "425 Can't determine IP").ok();
                                continue;
                            }
                        };

                        let port = addr.port();
                        let p1 = port / 256;
                        let p2 = port % 256;

                        let ip_str = match ip {
                            std::net::IpAddr::V4(v4) => {
                                let o = v4.octets();
                                format!("{},{},{},{}", o[0], o[1], o[2], o[3])
                            }
                            _ => {
                                writeln!(stream, "425 IPv6 not supported").ok();
                                continue;
                            }
                        };

                        writeln!(
                            stream,
                            "227 Entering Passive Mode ({},{},{})",
                            ip_str, p1, p2
                        ).ok();

                        session.pasv_listener = Some(listener);
                    }
                    Err(_) => {
                        writeln!(stream, "425 Cannot open passive connection").ok();
                    }
                }
            }

            "PWD" => {
                let path = session.cwd.to_string_lossy();
                writeln!(stream, "257 \"{}\"", path).ok();
            }

            "CWD" => {
                if let Some(dir) = arg {
                    let new_path = session.cwd.join(dir);
                    if new_path.is_dir() {
                        session.cwd = new_path;
                        writeln!(stream, "250 Directory changed").ok();
                    } else {
                        writeln!(stream, "550 Failed to change directory").ok();
                    }
                }
            }

            "PORT" => {
                if let Some(arg) = arg {
                    if let Some(addr) = parse_port(arg) {
                        session.data_addr = Some(addr);
                        writeln!(stream, "200 PORT command successful").ok();
                    } else {
                        writeln!(stream, "500 Invalid PORT").ok();
                    }
                }
            }

            "NLST"| "LIST" => {
                writeln!(stream, "150 Opening data connection").ok();

                if let Some(mut data_stream) = open_data_connection(&mut session) {
                    if let Ok(entries) = fs::read_dir(&session.cwd) {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            writeln!(data_stream, "{}", name).ok();
                        }
                    }

                    writeln!(stream, "226 Transfer complete").ok();
                } else {
                    writeln!(stream, "425 No data connection").ok();
                }
            }

            "MLSD" => {
                writeln!(stream, "150 Opening data connection").ok();

                if let Some(mut data_stream) = open_data_connection(&mut session) {
                    if let Ok(entries) = fs::read_dir(&session.cwd) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let name = entry.file_name();
                            let name = name.to_string_lossy();

                            let metadata = match entry.metadata() {
                                Ok(m) => m,
                                Err(_) => continue,
                            };

                            // type
                            let file_type = if metadata.is_dir() {
                                "dir"
                            } else {
                                "file"
                            };

                            // size
                            let size = metadata.len();

                            // modify time
                            let modify = metadata.modified()
                                .ok()
                                .and_then(|t| format_ftp_time(t).ok())
                                .unwrap_or_else(|| "19700101000000".to_string());

                            // собираем строку
                            if file_type == "file" {
                                writeln!(
                                    data_stream,
                                    "type={};size={};modify={}; {}",
                                    file_type, size, modify, name
                                ).ok();
                            } else {
                                writeln!(
                                    data_stream,
                                    "type={};modify={}; {}",
                                    file_type, modify, name
                                ).ok();
                            }
                        }
                    }

                    writeln!(stream, "226 Transfer complete").ok();
                } else {
                    writeln!(stream, "425 No data connection").ok();
                }
            }

            "RETR" => {
                if let Some(file) = arg {
                    let path = session.cwd.join(file);

                    if let Ok(mut file) = fs::File::open(path) {
                        writeln!(stream, "150 Opening data connection").ok();

                        if let Some(mut data_stream) = open_data_connection(&mut session) {
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).ok();
                            data_stream.write_all(&buf).ok();

                            writeln!(stream, "226 Transfer complete").ok();
                        } else {
                            writeln!(stream, "425 No data connection").ok();
                        }
                    } else {
                        writeln!(stream, "550 File not found").ok();
                    }
                }
            }
            "STOR" => {
                if let Some(file) = arg {
                    let path = session.cwd.join(file);

                    writeln!(stream, "150 Ok to receive data").ok();

                    if let Some(mut data_stream) = open_data_connection(&mut session) {
                        if let Ok(mut file) = fs::File::create(path) {
                            let mut buf = Vec::new();
                            data_stream.read_to_end(&mut buf).ok();
                            file.write_all(&buf).ok();

                            writeln!(stream, "226 Transfer complete").ok();
                        }
                    } else {
                        writeln!(stream, "425 No data connection").ok();
                    }
                }
            }

            "QUIT" => {
                writeln!(stream, "221 Goodbye").ok();
                break;
            }

            _ => {
                writeln!(stream, "502 Command not implemented").ok();
            }
        }
    }
}

fn parse_port(s: &str) -> Option<SocketAddr> {
    let nums: Vec<u16> = s.split(',').filter_map(|x| x.parse().ok()).collect();
    if nums.len() != 6 {
        return None;
    }

    let ip = format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3]);
    let port = nums[4] * 256 + nums[5];

    format!("{}:{}", ip, port).parse().ok()
}