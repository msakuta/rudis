import { createClient } from 'redis';

const client = createClient({
    url: 'redis://127.0.0.1:7878/'
});
client.on('error', err => console.log('Redis Client Error', err));
const con = await client.connect();

while(true) {
    console.log("Wait 1000ms");
    await (new Promise((resolve) => setTimeout(resolve, 1000)));
    console.log("Sending message");
    await client.publish("channel", "hey from deno");
}
