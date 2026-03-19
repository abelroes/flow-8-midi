use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const SINGLE_INSTANCE_ADDR: &str = "127.0.0.1:42187";
const SHOW_EXISTING_INSTANCE_MSG: &[u8] = b"show-existing-window";

pub enum Startup {
    Primary(mpsc::Receiver<()>),
    SecondaryInstanceNotified,
}

/// Starts a tiny loopback IPC channel used to keep a single app instance alive.
///
/// The first instance binds the address and listens for "show window" requests.
/// Any later instance simply connects, notifies the first one, and exits.
pub fn start() -> Result<Startup, String> {
    match TcpListener::bind(SINGLE_INSTANCE_ADDR) {
        Ok(listener) => start_primary(listener),
        Err(first_bind_err) => match notify_existing_instance() {
            Ok(true) => Ok(Startup::SecondaryInstanceNotified),
            Ok(false) => TcpListener::bind(SINGLE_INSTANCE_ADDR)
                .map_err(|second_bind_err| {
                    format!(
                        "Single-instance startup failed: bind error: {}; retry bind error: {}",
                        first_bind_err, second_bind_err
                    )
                })
                .and_then(start_primary),
            Err(connect_err) => Err(format!(
                "Single-instance startup failed: bind error: {}; notify error: {}",
                first_bind_err, connect_err
            )),
        },
    }
}

fn start_primary(listener: TcpListener) -> Result<Startup, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to configure single-instance listener: {}", e))?;

    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name("flow8-single-instance".to_string())
        .spawn(move || loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buf = [0u8; 32];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 && &buf[..n] == SHOW_EXISTING_INSTANCE_MSG => {
                            let _ = tx.send(());
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(200));
                }
                Err(_) => break,
            }
        })
        .map_err(|e| format!("Failed to spawn single-instance listener thread: {}", e))?;

    Ok(Startup::Primary(rx))
}

fn notify_existing_instance() -> Result<bool, String> {
    let mut stream = match TcpStream::connect(SINGLE_INSTANCE_ADDR) {
        Ok(stream) => stream,
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => return Ok(false),
        Err(e) => return Err(format!("Could not contact existing instance: {}", e)),
    };

    stream
        .write_all(SHOW_EXISTING_INSTANCE_MSG)
        .map_err(|e| format!("Could not notify existing instance: {}", e))?;

    Ok(true)
}
