# rudis

A fake redis server in Rust.

The server publishes a channel named "channel" periodically.

It is a coding practice to make a redis compatible server.
Do not use this in production.

## How to run the server

```
cargo r --bin serve
```

## How to run the test client

Note that the client is not the main target of this project, so it uses [redis crate](https://github.com/redis-rs/redis-rs).

```
cargo r --bin client
```
