use anyhow::{bail, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct ControlFlow {
    reader: BufReader<TcpStream>,
}

impl ControlFlow {
    pub fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let mut reader = BufReader::new(stream);

        // 220
        let _ = Self::read_response(&mut reader)?;

        Ok(Self { reader })
    }

    fn stream(&mut self) -> &mut TcpStream {
        self.reader.get_mut()
    }

    fn send_cmd(&mut self, cmd: &str) -> anyhow::Result<()> {
        let stream = self.stream();

        stream.write_all(format!("{}\r\n", cmd).as_bytes())?;
        stream.flush()?;

        Ok(())
    }

    fn read_response(reader: &mut BufReader<TcpStream>) -> Result<(u32, Vec<String>)> {
        let mut lines = Vec::new();

        let mut first = String::new();
        reader.read_line(&mut first)?;
        let code: u32 = first[0..3].parse().unwrap();

        let multiline = first.as_bytes()[3] == b'-';
        lines.push(first.clone());

        if multiline {
            loop {
                let mut line = String::new();
                reader.read_line(&mut line)?;
                if line.starts_with(&format!("{} ", code)) {
                    lines.push(line);
                    break;
                }
                lines.push(line);
            }
        }

        Ok((code, lines))
    }

    fn read(&mut self) -> Result<(u32, Vec<String>)> {
        Self::read_response(&mut self.reader)
    }

    pub fn user(&mut self, user: &str) -> Result<()> {
        self.send_cmd(&format!("USER {}", user))?;
        let a = self.read()?;
        Ok(())
    }

    pub fn pass(&mut self, pass: &str) -> Result<()> {
        self.send_cmd(&format!("PASS {}", pass))?;
        let a = self.read()?;
        Ok(())
    }

    pub fn type_i(&mut self) -> Result<()> {
        self.send_cmd("TYPE I")?;
        self.read()?;
        Ok(())
    }

    pub fn quit(&mut self) -> Result<()> {
        self.send_cmd("QUIT")?;
        self.read()?;
        Ok(())
    }


    fn enter_pasv(&mut self) -> Result<String> {
        self.send_cmd("PASV")?;
        let (_, lines) = self.read()?;

        let line = &lines[0];
        let start = line.find('(').unwrap();
        let end = line.find(')').unwrap();

        let nums: Vec<u16> = line[start + 1..end]
            .split(',')
            .map(|x| x.parse().unwrap())
            .collect();

        let ip = format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3]);
        let port = nums[4] * 256 + nums[5];

        Ok(format!("{}:{}", ip, port).parse()?)
    }


    pub fn list(&mut self) -> Result<Vec<u8>> {
        let addr = self.enter_pasv()?;
        let mut data_stream = TcpStream::connect(addr)?;

        self.send_cmd("MLSD")?;
        let (code, message) = self.read()?; // 150|125
        if(code>=200){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        let mut data = Vec::new();
        data_stream.read_to_end(&mut data)?;

        let (code, message) = self.read()?; // 226
        if(code!=226){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        Ok(data)
    }

    pub fn retr(&mut self, file: &str) -> Result<Vec<u8>> {
        let addr = self.enter_pasv()?;
        let mut data_stream = TcpStream::connect(addr)?;

        self.send_cmd(&format!("RETR {}", file))?;
        let (code, message) = self.read()?; // 150
        if(code!=150){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        let mut data = Vec::new();
        data_stream.read_to_end(&mut data)?;

        let (code, message) = self.read()?; // 226
        if(code!=226){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        Ok(data)
    }


    pub fn stor(&mut self, file: &str, data: &[u8]) -> Result<()> {
        let addr = self.enter_pasv()?;
        // let addr = format!("{}:{}", ip, port);
        let mut data_stream = TcpStream::connect(addr)?;

        self.send_cmd(&format!("STOR {}", file))?;
        let (code, message) = self.read()?; // 150
        if(code!=150){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        data_stream.write_all(data)?;
        drop(data_stream);
        let (code, message) = self.read()?; // 226
        if(code!=226){
            bail!("Received code {}: {}", code, message.join("\n"));
        }
        Ok(())
    }

    pub fn cwd(&mut self, path: &str) -> anyhow::Result<()> {
        self.send_cmd(&format!("CWD {}", path))?;

        let (code, _lines) = self.read()?;

        if code != 250 {
            anyhow::bail!("CWD failed with code {}", code);
        }

        Ok(())
    }

    pub fn pwd(&mut self) -> anyhow::Result<String> {
        self.send_cmd("PWD")?;

        let (code, lines) = self.read()?;

        if code != 257 {
            anyhow::bail!("PWD failed");
        }

        let line = &lines[0];
        let start = line.find('"').unwrap();
        let end = line[start + 1..].find('"').unwrap() + start + 1;

        Ok(line[start + 1..end].to_string())
    }

    pub fn dele(&mut self, file: &str) -> Result<()> {
        self.send_cmd(&format!("DELE {}", file))?;

        let (code, message) = self.read()?;

        if code != 250 {
            bail!("DELE failed with code {}: {}", code, message.join("\n"));
        }

        Ok(())
    }

    pub fn modify(&mut self, file: &str, data: Vec<u8>) -> Result<()> {
        self.dele(file)?;

        self.stor(file, &data)
    }
}
