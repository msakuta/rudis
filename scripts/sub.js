import { createClient } from 'redis';

const client = createClient({
    url: 'redis://127.0.0.1:7878/'
});
client.on('error', err => console.log('Redis Client Error', err));
const con = await client.connect();

console.log("Subscribing message");
await client.subscribe("channel", (msg) => {
    console.log(`Recved: ${msg}`);
});
