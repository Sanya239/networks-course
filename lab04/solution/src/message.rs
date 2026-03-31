use httparse::Header;

pub trait MessageParser: Send {
    fn parse(&mut self, buf: &[u8]) -> Result<Option<usize>, httparse::Error>;

    fn content_length(&self) -> usize;
}

pub fn get_content_length(headers: &[Header]) -> usize {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}
