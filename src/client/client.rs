//! A redis client to test the server.
//! Note that it uses redis crate for the sake of testing since we focus on implementing the server.

// use std::time::Duration;

// use redis::Commands;

use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn main() {
    tcp_client().unwrap();
    // do_something().unwrap();
}

fn tcp_client() -> Result<(), Box<dyn std::error::Error>> {
    let mut con = TcpStream::connect("localhost:7878")?;
    println!("Asking to GET hey");
    con.write_all(b"*2\r\n+GET\r\n+hey\r\n")?;
    let mut buf = [0u8; 128];
    let len = con.read(&mut buf)?;
    let resp = std::str::from_utf8(&buf[..len]);
    println!("Got some answer: {resp:?}");

    println!("Asking to SET hey");
    con.write_all(b"*3\r\n+SET\r\n+hey\r\n+42\r\n")?;
    let len = con.read(&mut buf)?;
    let resp = std::str::from_utf8(&buf[..len]);
    println!("Got some answer: {resp:?}");

    con.write_all(b"*2\r\n+GET\r\n+hey\r\n")?;
    let len = con.read(&mut buf)?;
    let resp = std::str::from_utf8(&buf[..len]);
    println!("Got some answer: {resp:?}");

    con.write_all(b"*2\r\n+SUBSCRIBE\r\n+channel\r\n")?;
    loop {
        let len = con.read(&mut buf)?;
        let resp = std::str::from_utf8(&buf[..len]);
        println!("Got some answer: {resp:?}");
    }
    unreachable!()
}

#[allow(dead_code)]
fn do_something() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:7878/")?;
    let mut con = client.get_connection()?;
    let mut pubsub = con.as_pubsub();
    pubsub.subscribe("channel")?;

    // let mut value = 0;
    for _ in 0.. {
        // let foo: String = con.get("foo")?;
        // println!("foo: {foo:?}");

        // con.set("foo", format!("bar{value}"))?;
        // value += 1;

        let msg = pubsub.get_message()?;
        let payload: String = msg.get_payload()?;
        println!("msg: {payload:?}");

        // std::thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
