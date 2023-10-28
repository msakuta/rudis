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
You can test a basic get/set, publishing or subscription to a topic name "channel".

```
cargo r --bin client -- {basic|pub|sub}
```

## How to run the test client in JavaScript

There is another test client in JavaScript in order to check conformance to the protocol.

First, install [node.js](https://nodejs.org/en) and [npm](https://www.npmjs.com/package/npm).

Then, install the dependency (redis package):

```
npm i
```

Then you can run the script for the periodic publishing test:

```
node scripts/pub.js
```

or subscription test:

```
node scripts/sub.js
```
