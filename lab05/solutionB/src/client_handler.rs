use crate::cmd::exec;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub async fn handle_client(mut client_stream: TcpStream) {
    println!(
        "received connection from {}",
        client_stream.peer_addr().unwrap()
    );
    let mut request = match HttpRequest::parse(&mut client_stream).await {
        Ok(v) => v,
        Err(err) => {
            println!("error parsing HTTP request: {}", err);
            respond_with_error(&mut client_stream, err.to_string(), "400".into()).await;
            return;
        }
    };

    let response = make_response(&mut request).await;
    let response = match response {
        Ok(v) => v,
        Err(err) => {
            println!("error making response: {}", err);
            respond_with_error(&mut client_stream, err.to_string(), "404".into()).await;
            return;
        }
    };
    let a = String::from_utf8(response.to_bytes().to_vec()).unwrap();
    let res = client_stream
        .write_all(response.to_bytes().as_slice())
        .await;
    client_stream.flush().await.unwrap();
}

async fn make_response(request: &mut HttpRequest) -> anyhow::Result<HttpResponse> {
    Ok(HttpResponse::from_body_ok(
        exec(request.get_body()).await?.into_bytes(),
    ))
}

async fn respond_with_error(client_stream: &mut TcpStream, message: String, code: String) {
    let body = format!(
        "<html>\
            <body>\
                <h1>{}</h1>\
                <p>{}</p>\
            </body>\
        </html>",
        code, message
    );

    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        code,
        body.len(),
        body
    );

    if let Err(e) = client_stream.write_all(response.as_bytes()).await {
        eprintln!("failed to send error response: {e}");
    }
}
