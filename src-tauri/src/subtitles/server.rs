//! The local web server that an OBS browser source loads the subtitles from.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::hub::{Hub, Message};

const PAGE: &str = include_str!("page.html");

/// How often the accept loop looks for a new connection and for a request to stop.
const ACCEPT_INTERVAL: Duration = Duration::from_millis(50);

/// How often an open event stream looks for a new message and for a request to stop.
const STREAM_INTERVAL: Duration = Duration::from_millis(250);

/// How long a client may take to send its request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// How long an event stream may stay silent before a comment keeps the connection alive.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

/// The most that the request line and the headers of a request may take.
const MAX_REQUEST_HEAD_BYTES: u64 = 8 * 1024;

/// Serves the subtitles page and its event stream on the loopback address.
///
/// Dropping the server stops it and closes the open event streams.
pub struct Server {
    port: u16,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Starts listening on `port`, or on a free port when it is 0.
    pub fn start(port: u16, hub: Arc<Hub>) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let stopped = Arc::new(AtomicBool::new(false));

        let thread = thread::spawn({
            let stopped = Arc::clone(&stopped);
            move || accept(&listener, port, &hub, &stopped)
        });

        Ok(Self {
            port,
            stopped,
            thread: Some(thread),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// What a client asked for.
struct Request {
    method: String,
    path: String,
    host: Option<String>,
}

enum Route {
    Page,
    Events,
    Rejected(&'static str),
}

fn accept(listener: &TcpListener, port: u16, hub: &Arc<Hub>, stopped: &Arc<AtomicBool>) {
    while !stopped.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let hub = Arc::clone(hub);
                let stopped = Arc::clone(stopped);
                thread::spawn(move || {
                    // A client that goes away mid request needs no answer.
                    let _ = serve(stream, port, &hub, &stopped);
                });
            }
            Err(_) => thread::sleep(ACCEPT_INTERVAL),
        }
    }
}

fn serve(mut stream: TcpStream, port: u16, hub: &Hub, stopped: &AtomicBool) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;

    let answer = match read_request(&stream)? {
        Some(request) => route(&request, port),
        None => Route::Rejected("400 Bad Request"),
    };
    match answer {
        Route::Page => respond(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE),
        Route::Events => stream_events(&mut stream, hub, stopped),
        Route::Rejected(status) => {
            respond(&mut stream, status, "text/plain; charset=utf-8", status)
        }
    }
}

/// Reads the request line and headers, or returns `None` when they are malformed or too long.
fn read_request(stream: &TcpStream) -> io::Result<Option<Request>> {
    let mut reader = BufReader::new(Read::take(stream, MAX_REQUEST_HEAD_BYTES));

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        return Ok(None);
    };

    let mut host = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("host")
        {
            host = Some(value.trim().to_owned());
        }
    }

    let path = target.split('?').next().unwrap_or(target);
    Ok(Some(Request {
        method: method.to_owned(),
        path: path.to_owned(),
        host,
    }))
}

/// Decides the answer. A foreign host is refused so that a web page cannot reach the server by
/// pointing its own name at this computer.
fn route(request: &Request, port: u16) -> Route {
    let own_hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    let is_own_host = request
        .host
        .as_deref()
        .is_some_and(|host| own_hosts.iter().any(|own| host.eq_ignore_ascii_case(own)));
    if !is_own_host {
        return Route::Rejected("403 Forbidden");
    }
    if request.method != "GET" {
        return Route::Rejected("405 Method Not Allowed");
    }
    match request.path.as_str() {
        "/" => Route::Page,
        "/events" => Route::Events,
        _ => Route::Rejected("404 Not Found"),
    }
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
    stream.write_all(response.as_bytes())
}

/// Sends each message as it is published, until the client leaves or the server stops.
fn stream_events(stream: &mut TcpStream, hub: &Hub, stopped: &AtomicBool) -> io::Result<()> {
    // Subscribing first means no message falls between the connection and the stream.
    let messages = hub.subscribe();
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\nretry: 1000\n\n",
    )?;

    let mut last_write = Instant::now();
    while !stopped.load(Ordering::Relaxed) {
        match messages.recv_timeout(STREAM_INTERVAL) {
            Ok(message) => {
                stream.write_all(format_event(&message).as_bytes())?;
                last_write = Instant::now();
            }
            Err(RecvTimeoutError::Timeout) => {
                if last_write.elapsed() >= KEEPALIVE_INTERVAL {
                    stream.write_all(b": keepalive\n\n")?;
                    last_write = Instant::now();
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn format_event(message: &Message) -> String {
    match message {
        Message::Speaking(text) => {
            format!(
                "event: speaking\ndata: {}\n\n",
                serde_json::json!({ "text": text })
            )
        }
        Message::Ended => "event: ended\ndata: {}\n\n".to_owned(),
        Message::Cancelled => "event: cancelled\ndata: {}\n\n".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A client that keeps what the server sent, so text it waits for may arrive in pieces.
    struct Client {
        stream: TcpStream,
        received: String,
    }

    impl Client {
        fn connect(port: u16, request_line: &str, host: &str) -> Self {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            write!(
                stream,
                "{request_line} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            Self {
                stream,
                received: String::new(),
            }
        }

        fn open_events(port: u16) -> Self {
            let mut client = Self::connect(port, "GET /events", &format!("127.0.0.1:{port}"));
            client.wait_for("retry: 1000\n\n");
            client
        }

        /// Waits until `expected` has arrived and forgets everything up to the end of it.
        fn wait_for(&mut self, expected: &str) {
            loop {
                if let Some(index) = self.received.find(expected) {
                    self.received.drain(..index + expected.len());
                    return;
                }
                assert!(
                    self.receive_more(),
                    "the stream ended before {expected:?} arrived, only {:?} did",
                    self.received
                );
            }
        }

        fn read_to_close(&mut self) -> String {
            while self.receive_more() {}
            std::mem::take(&mut self.received)
        }

        /// Returns false when the server closed the connection.
        fn receive_more(&mut self) -> bool {
            let mut chunk = [0; 1024];
            let count = self.stream.read(&mut chunk).unwrap();
            self.received
                .push_str(&String::from_utf8_lossy(&chunk[..count]));
            count > 0
        }
    }

    fn start() -> (Server, Arc<Hub>) {
        let hub = Arc::new(Hub::default());
        let server = Server::start(0, Arc::clone(&hub)).unwrap();
        (server, hub)
    }

    #[test]
    fn requests_are_answered_by_what_they_ask_for_and_who_asks() {
        let (server, _hub) = start();
        let port = server.port();
        let own_host = format!("127.0.0.1:{port}");
        let cases = [
            ("GET /", own_host.as_str(), "200 OK"),
            ("GET /?hold=8", &format!("localhost:{port}"), "200 OK"),
            ("GET /missing", own_host.as_str(), "404 Not Found"),
            ("POST /", own_host.as_str(), "405 Method Not Allowed"),
            ("GET /", "example.com", "403 Forbidden"),
        ];

        for (request_line, host, expected_status) in cases {
            let response = Client::connect(port, request_line, host).read_to_close();

            assert!(
                response.starts_with(&format!("HTTP/1.1 {expected_status}\r\n")),
                "{request_line} from {host} got {response}"
            );
        }
    }

    #[test]
    fn events_reach_every_client_and_end_when_the_server_stops() {
        let (server, hub) = start();
        let mut early = Client::open_events(server.port());

        hub.publish(Message::Speaking("Hello \"world\"".to_owned()));
        let speaking = "event: speaking\ndata: {\"text\":\"Hello \\\"world\\\"\"}\n\n";
        early.wait_for(speaking);

        let mut late = Client::open_events(server.port());
        late.wait_for(speaking);

        hub.publish(Message::Ended);
        early.wait_for("event: ended\ndata: {}\n\n");

        hub.publish(Message::Cancelled);
        early.wait_for("event: cancelled\ndata: {}\n\n");

        drop(server);
        assert_eq!(early.read_to_close(), "");
    }
}
