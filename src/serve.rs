mod ser;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Duration,
};

use self::ser::{serialize_bulk_str, serialize_str, serialize_null};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    let server = Arc::new(Mutex::new(Server::default()));

    let mut thread_id = 0;

    let server_clone = server.clone();
    std::thread::spawn(move || clock(server_clone));

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

    'parse_commands: loop {
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
                break 'parse_commands;
            };
            match command {
                "subscribe" => {
                    let rx = server
                        .lock()
                        .ok()
                        .zip(req.get(1).and_then(RedisValue::as_str))
                        .map(|(mut server_lock, sub)| {
                            let (tx, rx) = std::sync::mpsc::channel();

                            server_lock
                                .subscribers
                                .entry(sub.to_string())
                                .or_default()
                                .push(tx);

                            if let Err(e) = (|| -> std::io::Result<()> {
                                reader.write_all(b"*3\r\n")?;
                                serialize_bulk_str(&mut reader, "subscribe")?;
                                serialize_bulk_str(&mut reader, sub)?;
                                reader.write_all(b":1\r\n")?;
                                Ok(())
                            })() {
                                println!("Error: {e:?}");
                            }
                            rx
                        });
                    if let Some(rx) = rx {
                        subscribe_loop(&mut reader, &server, rx).unwrap();
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
                    con.write_all(b"*3\r\n")?;
                    serialize_bulk_str(con, "message")?;
                    serialize_bulk_str(con, channel)?;
                    serialize_bulk_str(con, &data)?;
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

#[derive(Debug)]
enum RedisValue {
    Str(String),
    Array(Vec<RedisValue>),
    Map(HashMap<String, RedisValue>),
}

impl RedisValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}

fn deserialize(f: &mut impl Read) -> Result<RedisValue, Box<dyn std::error::Error>> {
    let tag = read_byte(f)?;
    match tag {
        b'$' => {
            // string
            let len = parse_len(f)?;
            println!("str len: {len}");
            let mut str_buf = vec![0u8; len];
            f.read_exact(&mut str_buf)?;
            if !matches!(read_byte(f), Ok(b'\r')) || !matches!(read_byte(f), Ok(b'\n')) {
                return Err("String not followed by a CRLF".into());
            }
            let str = String::from_utf8(str_buf)?;
            println!("Str(utf-8): {str}");
            Ok(RedisValue::Str(str))
        }
        b'+' => {
            // simple string
            let mut str_buf = vec![0u8; 2];
            f.read_exact(&mut str_buf)?;
            loop {
                if str_buf[str_buf.len() - 2..] == b"\r\n"[..] {
                    return Ok(RedisValue::Str(
                        std::str::from_utf8(&str_buf[..str_buf.len() - 2])?.to_string(),
                    ));
                }
                let mut next_buf = [0u8; 1];
                let len = f.read(&mut next_buf)?;
                str_buf.extend_from_slice(&next_buf[..len]);
            }
        }
        b'*' => {
            // array
            let len = parse_len(f)?;
            println!("array len: {len}");
            let v = (0..len).map(|_| deserialize(f)).collect::<Result<_, _>>()?;
            Ok(RedisValue::Array(v))
        }
        b'%' => {
            // map
            let len = parse_len(f)?;
            println!("map len: {len}");
            let mut hash = HashMap::new();
            for _ in 0..len / 2 {
                let RedisValue::Str(k) = deserialize(f)? else {
                    return Err("Key is not a string".into());
                };
                println!("key: {k:?}");
                let v = deserialize(f)?;
                hash.insert(k, v);
            }
            Ok(RedisValue::Map(hash))
        }
        _ => Err(format!("Not formatted as redis value: tag was {}", tag as char).into()),
    }
}

fn parse_len(f: &mut impl Read) -> Result<usize, Box<dyn std::error::Error>> {
    let mut len_buf = [b'0'; 16];
    let mut i = 0;
    let mut first_byte = None;
    while let Ok(b) = read_byte(f) {
        if matches!(b, b'0'..=b'9') {
            len_buf[i] = b;
            i += 1;
        } else {
            first_byte = Some(b);
            break;
        }
    }
    if !matches!(first_byte, Some(b'\r')) || !matches!(read_byte(f), Ok(b'\n')) {
        return Err("Length not followed by a CRLF".into());
    }
    let len = std::str::from_utf8(&len_buf[..i])?.parse::<usize>()?;
    Ok(len)
}

fn read_byte(f: &mut impl Read) -> std::io::Result<u8> {
    let mut buf = [0u8];
    f.read_exact(&mut buf)?;
    Ok(buf[0])
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
