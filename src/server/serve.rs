use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    let subscribers = Arc::new(Mutex::new(HashMap::new()));

    let mut thread_id = 0;

    let subs = subscribers.clone();
    std::thread::spawn(move || clock(subs));

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let subs = subscribers.clone();
        std::thread::spawn(move || process_thread(stream, thread_id, subs));
        thread_id += 1;
    }
}

fn clock(subscribers: Arc<Mutex<Subscribers>>) {
    let mut time = 0;
    loop {
        'lock: {
            let Ok(lock) = subscribers.lock() else {
                break 'lock;
            };
            let Some(channel) = lock.get("channel") else {
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

type Subscribers = HashMap<String, Vec<std::sync::mpsc::Sender<String>>>;

fn process_thread(stream: TcpStream, _thread_id: usize, subscribers: Arc<Mutex<Subscribers>>) {
    println!("Connection established!");

    let mut reader = stream;
    println!("Connection established!");

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

    let redis_request = match deserialize(&mut reader) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Deserialize error: {e:?}");
            return;
        }
    };

    println!("Request: {redis_request:?}");

    let (tx, rx) = std::sync::mpsc::channel();

    if let RedisValue::Map(map) = redis_request {
        if let Some(sub) = map.get("SUBSCRIBE").and_then(RedisValue::as_str) {
            if let Ok(mut subscribers) = subscribers.lock() {
                subscribers.entry(sub.to_string()).or_default().push(tx);

                if let Err(e) = (|| -> std::io::Result<()> {
                    reader.write_all(b"*3\r\n")?;
                    serialize_bulk_str(&mut reader, "subscribe")?;
                    serialize_bulk_str(&mut reader, sub)?;
                    reader.write_all(b":1\r\n")?;
                    Ok(())
                })() {
                    println!("Error: {e:?}");
                }
            }
        }
    }

    while let Ok(data) = rx.recv() {
        println!("recv: {data:?}");
        if let Ok(subscribers) = subscribers.lock() {
            for (channel, _) in subscribers.iter() {
                if let Err(e) = (|| -> std::io::Result<()> {
                    reader.write_all(b"*3\r\n")?;
                    serialize_bulk_str(&mut reader, "message")?;
                    serialize_bulk_str(&mut reader, channel)?;
                    serialize_bulk_str(&mut reader, &data)?;
                    Ok(())
                })() {
                    println!("Error: {e:?}");
                }
            }
        }
    }

    // stream.write_all(SAMPLE_RESPONSE);
}

#[allow(dead_code)]
fn serialize_bulk_str(f: &mut impl std::io::Write, s: &str) -> std::io::Result<()> {
    write!(f, "${}\r\n", s.len())?;
    write!(f, "{}\r\n", s)?;
    Ok(())
}

fn serialize_str(f: &mut impl std::io::Write, s: &str) -> std::io::Result<()> {
    write!(f, "+{}\r\n", s)?;
    Ok(())
}

#[allow(dead_code)]
fn serialize_map(
    f: &mut impl std::io::Write,
    map: &HashMap<String, String>,
) -> std::io::Result<()> {
    write!(f, "*{}\r\n", map.len())?;
    for (k, v) in map {
        serialize_str(f, k)?;
        serialize_str(f, v)?;
    }
    Ok(())
}

#[derive(Debug)]
enum RedisValue {
    Str(String),
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
        b'*' => {
            let len = parse_len(f)?;
            println!("len: {len}");
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
