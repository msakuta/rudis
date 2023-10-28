mod de;
mod redis_value;
mod ser;

use std::{
    collections::HashMap,
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Duration,
};

use self::{
    de::deserialize,
    redis_value::RedisValue,
    ser::{serialize_array, serialize_null, serialize_str},
};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    let server = Arc::new(Mutex::new(Server::default()));

    let mut thread_id = 0;

    // let server_clone = server.clone();
    // std::thread::spawn(move || clock(server_clone));

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let server_clone = server.clone();
        std::thread::spawn(move || process_thread(stream, thread_id, server_clone));
        thread_id += 1;
    }
}

type Subscribers = HashMap<String, Vec<std::sync::mpsc::Sender<String>>>;

#[derive(Default)]
struct Server {
    subscribers: Subscribers,
    data: HashMap<String, String>,
}

fn clock(server: Arc<Mutex<Server>>) {
    let mut time = 0;
    loop {
        'lock: {
            let Ok(lock) = server.lock() else {
                break 'lock;
            };
            let Some(channel) = lock.subscribers.get("channel") else {
                break 'lock;
            };
            for tx in channel {
                if let Err(e) = tx.send(format!("Hello {}", time)) {
                    eprintln!("Error writing a sender: {e:?}");
                }
            }
        }
        time += 1;
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn process_thread(stream: TcpStream, _thread_id: usize, server: Arc<Mutex<Server>>) {
    println!("Connection established!");

    let mut reader = stream;

    // let mut buf = vec![0u8; 32];
    // while let Ok(_) = reader.read(&mut buf) {
    //     match std::str::from_utf8(&buf) {
    //         Ok(s) => println!("read: {s:?}"),
    //         Err(_) => println!("read(non-utf8): {buf:?}"),
    //     }
    // }
    // let mut all = String::new();
    // let Ok(_) = reader.read_to_string(&mut all) else { continue };
    // println!("Request: {all:?}");
    // let mut line = String::new();
    // let Ok(redis_request) = reader
    //     .read_line(&mut line) else { continue; };

    // println!("Request: {redis_request:?}");

    loop {
        let redis_request = match deserialize(&mut reader) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Deserialize error: {e:?}");
                return;
            }
        };

        println!("Request: {redis_request:?}");

        if let RedisValue::Array(req) = redis_request {
            let Some(command) = req.get(0).and_then(RedisValue::as_str) else {
                eprintln!("Client request didn't start with a string");
                continue;
            };
            match &command.to_uppercase() as &_ {
                "SUBSCRIBE" => {
                    if let Err(e) = subscribe(&mut reader, &req, &server) {
                        respond_error(&mut reader, &e.to_string());
                    }
                }
                "PUBLISH" => {
                    if let Err(e) = publish(&mut reader, &req, &server) {
                        respond_error(&mut reader, &e.to_string());
                    }
                }
                "GET" => {
                    if let Some((server, key)) = server
                        .lock()
                        .ok()
                        .zip(req.get(1).and_then(RedisValue::as_str))
                    {
                        if let Some(value) = server.data.get(key) {
                            if let Err(e) = serialize_str(&mut reader, value) {
                                eprintln!("Error: {e:?}");
                            }
                        } else {
                            if let Err(e) = serialize_null(&mut reader) {
                                eprintln!("Error: {e:?}");
                            }
                            println!("Wrote null");
                        }
                    }
                }
                "SET" => {
                    if let Some(((mut server, key), value)) = server
                        .lock()
                        .ok()
                        .zip(req.get(1).and_then(RedisValue::as_str))
                        .zip(req.get(2).and_then(RedisValue::as_str))
                    {
                        server.data.insert(key.to_string(), value.to_string());
                        if let Err(e) = serialize_str(&mut reader, "OK") {
                            eprintln!("Error: {e:?}");
                        }
                    }
                }
                _ => eprintln!("Unknown command: {command:?}"),
            }
        }
    }
}

fn subscribe(
    con: &mut TcpStream,
    req: &[RedisValue],
    server: &Arc<Mutex<Server>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut server_lock = server.lock().map_err(|e| e.to_string())?;
    let Some(sub) = req.get(1).and_then(RedisValue::as_str) else {
        return Err("Lacking channel name".into());
    };
    let (tx, rx) = std::sync::mpsc::channel();

    server_lock
        .subscribers
        .entry(sub.to_string())
        .or_default()
        .push(tx);

    serialize_array(
        con,
        &[
            RedisValue::from("subscribe"),
            RedisValue::from(sub),
            RedisValue::Integer(1),
        ],
    )?;
    drop(server_lock);
    subscribe_loop(con, &server, rx).unwrap();
    Ok(())
}

fn subscribe_loop(
    con: &mut TcpStream,
    server: &Arc<Mutex<Server>>,
    rx: Receiver<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok(data) = rx.recv() {
        println!("recv: {data:?}");
        if let Ok(server) = server.lock() {
            for (channel, _) in server.subscribers.iter() {
                if let Err(e) = (|| -> std::io::Result<()> {
                    serialize_array(con, &["message", channel, &data])?;
                    Ok(())
                })() {
                    println!("Error: {e:?}");
                }
            }
        }
    }

    // stream.write_all(SAMPLE_RESPONSE);
    Ok(())
}

fn publish(
    con: &mut TcpStream,
    req: &[RedisValue],
    server: &Arc<Mutex<Server>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(channel) = req.get(1).and_then(RedisValue::as_str) else {
        return Err("Lacking channel name".into());
    };
    let server_lock = server
        .try_lock()
        .map_err(|e| format!("Server lock failed {e:?}"))?;
    let Some(message) = req.get(2).and_then(RedisValue::as_str) else {
        return Err("Message was not supplied or was not a string".into());
    };
    if let Some(sub) = server_lock.subscribers.get(channel) {
        eprintln!("publishing to {} subscribers...", sub.len());
        for tx in sub {
            if let Err(e) = tx.send(message.to_string()) {
                eprintln!("Error sending a message to a subscriber: {e:?}");
            }
        }
        serialize_str(con, "OK")?;
    } else {
        eprintln!("There were no subscribers to {}", channel);
        // Having no subscribers is not an error.
        serialize_str(con, "OK")?;
    }
    Ok(())
}

fn respond_error(stream: &mut impl Write, msg: &str) {
    eprintln!("Response error: {msg}");
    // If we fail in an attempt to respond with an error, there is not much we can do but
    // print int to the server's terminal (or probably on a log).
    if let Err(e) = write!(stream, "-{msg}\r\n") {
        eprintln!("Error: {e:?}");
    }
}

#[allow(dead_code)]
const SAMPLE_RESPONSE: &str = r#"
*14
$6
server
$5
redis
$7
version
$5
7.2.2
$5
proto
:2
$2
id
:24
$4
mode
$10
standalone
$4
role
$6
master
$7
modules
*0
"#;
