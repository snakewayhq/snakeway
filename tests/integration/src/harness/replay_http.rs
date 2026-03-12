use crate::constants::TEST_HOST;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn normalize_http_newlines(req: Vec<u8>) -> Vec<u8> {
    // convert to string lossily for newline normalization
    let mut s = String::from_utf8_lossy(&req).into_owned();

    // normalize all line endings
    s = s.replace("\r\n", "\n");
    s = s.replace('\n', "\r\n");

    // ensure header terminator
    if !s.ends_with("\r\n\r\n") {
        if s.ends_with("\r\n") {
            s.push_str("\r\n");
        } else {
            s.push_str("\r\n\r\n");
        }
    }

    s.into_bytes()
}

pub fn replay_http_fixture(path: &str, port: u16) -> String {
    let path = format!("{}/{}", crate::constants::FIXTURES_HTTP_DIR, path);
    let req = fs::read(path).unwrap();
    let req = normalize_http_newlines(req);

    let mut stream = TcpStream::connect((TEST_HOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    stream.write_all(&req).unwrap();

    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];

    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => panic!("read error: {e}"),
        }
    }

    String::from_utf8_lossy(&buf).to_string()
}
