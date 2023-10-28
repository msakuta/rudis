//! A redis client to test the server.
//! Note that it uses redis crate for the sake of testing since we focus on implementing the server.

mod de;
mod redis_value;
mod ser;

use redis::Commands;

use self::{de::deserialize, ser::serialize_str_array};

use std::{net::TcpStream, time::Duration};

fn main() {
    let mut args = std::env::args();
    let program = args.next();
    let Some(app) = args.next() else {
        println!("usage: {} {{basic|sub|pub}}", program.unwrap());
        return;
    };
    match &app as &_ {
        "basic" => tcp_client().unwrap(),
        "sub" => simple_subscriber().unwrap(),
        "pub" => simple_publisher().unwrap(),
        _ => println!("Please specify basic, sub or pub"),
    }
}

fn tcp_client() -> Result<(), Box<dyn std::error::Error>> {
    let mut con = TcpStream::connect("localhost:7878")?;
    println!("Asking to GET hey");
    serialize_str_array(&mut con, &["GET", "hey"])?;
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    println!("Asking to SET hey");
    serialize_str_array(&mut con, &["SET", "hey", "42"])?;
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    println!("Asking to GET hey again");
    serialize_str_array(&mut con, &["GET", "hey"])?;
    let resp = deserialize(&mut con)?;
    println!("Got some answer: {resp:?}");

    serialize_str_array(&mut con, &["subscribe", "channel"])?;
    loop {
        let resp = deserialize(&mut con)?;
        println!("Got some answer: {resp:?}");
    }
    unreachable!()
}

#[allow(dead_code)]
fn simple_subscriber() -> redis::RedisResult<()> {
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

fn simple_publisher() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:7878/")?;
    let mut con = client.get_connection()?;
    loop {
        con.publish("channel", "Hey from Rust")?;
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
