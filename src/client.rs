//! A redis client to test the server.
//! Note that it uses redis crate for the sake of testing since we focus on implementing the server.

mod de;
mod redis_value;
mod ser;

use self::{de::deserialize, ser::serialize_array};

// use std::time::Duration;

// use redis::Commands;

use std::{io::Read, net::TcpStream};

fn main() {
    tcp_client().unwrap();
    // do_something().unwrap();
}

fn tcp_client() -> Result<(), Box<dyn std::error::Error>> {
    let mut con = TcpStream::connect("localhost:7878")?;
    println!("Asking to GET hey");
    serialize_array(&mut con, &["GET", "hey"])?;
    let mut buf = [0u8; 128];
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    println!("Asking to SET hey");
    serialize_array(&mut con, &["SET", "hey", "42"])?;
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    println!("Asking to GET hey again");
    serialize_array(&mut con, &["GET", "hey"])?;
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    serialize_array(&mut con, &["subscribe", "channel"])?;
    loop {
        let resp = deserialize(&mut con)?;
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
