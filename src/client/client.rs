//! A redis client to test the server.
//! Note that it uses redis crate for the sake of testing since we focus on implementing the server.

// use std::time::Duration;

// use redis::Commands;

fn main() {
    do_something().unwrap();
}

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
