use std::hash::Hash;
use std::net::{TcpStream, TcpListener};
use std::{result, thread};
use std::sync::{Arc, Mutex};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::collections::{hash_map, HashMap};

struct User {
    name: String,
    ip: String
}

struct Request {
    headers: HashMap<String, String>,
    body: String
}

fn handle_connection(mut stream: TcpStream, clients: Arc<Mutex<Vec<User>>>) {
    let mut reader = BufReader::new(&stream);
    
    let mut request_headers: HashMap<String, String> = HashMap::new();
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).unwrap();

        if line.trim().is_empty() || bytes_read == 0 {
            break;
        }

        if line.contains("HTTP") {
            continue;
        }

        let sep_line = line.split_once(": ").unwrap();

        request_headers.insert(sep_line.0.to_string(), sep_line.1.to_string());
    }
    let body;
    if request_headers.contains_key("Content-Length") {
        let size = request_headers["Content-Length"].trim().parse().unwrap();
        let mut buffer = reader.take(size);
        let mut body_vec: Vec<u8> = Vec::new();
        buffer.read_to_end(&mut body_vec).unwrap();
        body = String::from_utf8(body_vec).unwrap();
    } else {
        body = "N/A".to_string();
    }

    let request = Request {
        headers: request_headers,
        body: body
    };

    route(request, clients);
}

fn route(request: Request, clients: Arc<Mutex<Vec<User>>>) {
    
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();

    let clients: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(Vec::new()));
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let client_list_clone = Arc::clone(&clients);
        thread::spawn(move || {
            handle_connection(stream, client_list_clone);
        });
    }
}
