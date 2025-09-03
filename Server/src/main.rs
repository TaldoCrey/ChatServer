use std::hash::Hash;
use std::net::{TcpStream, TcpListener};
use std::os::unix::net::SocketAddr;
use std::{result, thread};
use std::sync::{Arc, Mutex};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::collections::{hash_map, HashMap};
use std::path::Path;
use chrono::{Utc, Local};
use percent_encoding::{percent_decode_str};

#[derive(Debug)]
struct User {
    name: String,
    ip: String,
    port: String
}

struct Request {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    body: String
}

impl Request {
    fn show(&self) {

        println!("Method: {}\nURI: {}\n", self.method, self.uri);

        for (k, v) in &self.headers {
            println!("{}: {}", k, v);
        }
        println!("---BODY---\n{}", self.body);
    }
}

fn parse(mut request_buffer: BufReader<&TcpStream>) -> Request {
    let mut request_headers: HashMap<String, String> = HashMap::new();
    let mut request_method = String::from("N/A");
    let mut request_uri = String::from("N/A");
    loop {
        let mut line = String::new();
        let bytes_read = request_buffer.read_line(&mut line).unwrap();

        if line.contains("HTTP") {
            request_method = line.split(" ").nth(0).unwrap_or("N/A").to_string();
            request_uri = line.split(" ").nth(1).unwrap_or("N/A").to_string();
            continue;
        }

        if line.trim().is_empty() || bytes_read == 0 {
            break;
        }

        let sep_line = line.split_once(": ").unwrap();

        request_headers.insert(sep_line.0.trim().to_string(), sep_line.1.trim().replace("\n", "").to_string());
    }

    let body;
    if request_headers.contains_key("Content-Length") {
        let size = request_headers["Content-Length"].trim().parse().unwrap();
        let mut buffer = request_buffer.take(size);
        let mut body_vec: Vec<u8> = Vec::new();
        buffer.read_to_end(&mut body_vec).unwrap();
        body = String::from_utf8(body_vec).unwrap();
    } else {
        body = "N/A".to_string();
    }

    Request {
        method: request_method,
        uri: request_uri,
        headers: request_headers,
        body: body
    }
}

fn handle_connection(mut stream: TcpStream, clients: Arc<Mutex<Vec<User>>>) {
    let reader = BufReader::new(&stream);
    
    let request = parse(reader);

    //println!("NEW REQUEST RECEIVED:\n");
    //request.show();
    let response = route(request, stream.peer_addr().unwrap().to_string(), clients);

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn convert_html_utf8(msg: String) -> String {
    let msg = percent_decode_str(&msg).decode_utf8_lossy().replace("+", " ").to_string();

    msg
}

fn route(request: Request, origin_ip: String, clients: Arc<Mutex<Vec<User>>>) -> String {
    if request.method == "GET" {
        if request.uri == "/" {
            let contents = fs::read_to_string("./pages/register.html").unwrap();
            format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/html\r\n\
                Content-Length: {}\r\n\
                \r\n\
                {}",
                contents.len(),
                contents
            )
        } else if request.uri == "/chat" {
            let mut registered = false;

            let connected_clients = clients.lock().unwrap();

            for client in connected_clients.iter() {
                if origin_ip.split_once(":").unwrap().0 == client.ip {
                    registered = true;
                    break;
                }
            }


            if registered {
                let contents = fs::read_to_string("./pages/index.html").unwrap();
                let messages = fs::read("./logs/messages.txt").unwrap();
                let messages = String::from_utf8(messages).unwrap();
                let contents = contents.replace("{{MESSAGES}}", messages.as_str());

                format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/html\r\n\
                    Content-Length: {}\r\n\
                    \r\n\
                    {}",
                    contents.len(),
                    contents
                )
            } else {
                format!(
                    "HTTP/1.1 302 MOVED PERMANENTLY\r\n\
                    Location:/\r\n\
                    \r\n"
                )
            }
            
        } else if request.uri == "/style.css" {
            let contents = fs::read_to_string("./pages/style.css").unwrap();
            format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/css\r\n\
                Content-Length: {}\r\n\
                \r\n\
                {}",
                contents.len(),
                contents
            )
        } else {
            format!(
                "HTTP/1.1 404 NOT FOUND\r\n\r\n"
            )
        }
    } else if request.method == "POST" {
        if request.uri == "/register-user" {
            let client_username = request.body.trim().split_once("=").unwrap().1.to_string();
            let mut clients_vec = clients.lock().unwrap();
            clients_vec.push(User {name: client_username, ip: origin_ip.split_once(":").unwrap().0.to_string(), port: origin_ip.split_once(":").unwrap().1.to_string()});
            println!("Clients connected: {:?}", clients_vec);
            format!(
                "HTTP/1.1 302 MOVED PERMANENTLY\r\n\
                Location:/chat\r\n\
                \r\n"
            )     
        } else if request.uri == "/send" {
            let mut clients_vec = clients.lock().unwrap();
            let mut client_name = "not_registered".to_string();
            for connected_client in clients_vec.iter() {
                if connected_client.ip == origin_ip.split_once(":").unwrap().0 {
                    client_name = connected_client.name.clone();
                }
            }

            if client_name != "not_registered" {
                let message = request.body.split_once("=").unwrap().1.to_string();

                let message = convert_html_utf8(message);

                println!("New message posted: {}", message);

                let mut messages_logs = fs::OpenOptions::new().append(true).create(true).open("./logs/messages.txt").unwrap();

                let timestamp = Local::now().format("%d/%m/%Y-%H:%M:%S");

                writeln!(messages_logs, "[{}] {}: {}", timestamp, client_name, message).unwrap();

                format!(
                    "HTTP/1.1 302 MOVED PERMANENTLY\r\n\
                    Location:/chat\r\n\
                    \r\n"
                )
            } else {
                format!(
                    "HTTP/1.1 302 MOVED PERMANENTLY\r\n\
                    Location:/\r\n\
                    \r\n"
                )
            }

        } else {
            format!(
                "HTTP/1.1 404 NOT FOUND\r\n\r\n"
            )
        }
    } else {
        format!(
            "HTTP/1.1 404 NOT FOUND\r\n\r\n"
        )
    }
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
