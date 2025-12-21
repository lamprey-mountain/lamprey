const ws = new WebSocket("ws://localhost:4000/api/v1/sync?version=1&compression=deflate");
ws.binaryType = "arraybuffer";

const setState = (s) => {
  console.log("state", s);
}

setState("reconnect");

const decomp = new DecompressionStream("deflate");
const dec = new TextDecoderStream();

const comp = new CompressionStream("deflate");
const enc = new TextEncoderStream();

const send = (msg) => {
  r.write(JSON.stringify(msg));
}

(async () => {
  for await (const chunk of enc.readable.pipeThrough(comp)) {
    console.log("write", chunk.byteLength);
    ws.send(chunk);
  }
})();

(async () => {
  for await (const chunk of decomp.readable.pipeThrough(dec)) {
    console.log("red decompressed", chunk.length);
    const msg = JSON.parse(chunk)
    // console.log(msg);

    if (msg.op === "Ping") {
      send({ type: "Pong" })
    } else if (msg.op === "Ready") {
      setState("connected");
    }
  }
})();

const r = enc.writable.getWriter();
ws.onopen = () => {
  send({
    type: "Hello",
    token: "4048dc96-a18c-4685-94c5-156545e843c3",
  });
};

const w = decomp.writable.getWriter();
ws.onmessage = (e) => {
  console.log("read", e);
  console.log("read", e.data.byteLength);
  w.write(e.data);
};

ws.onerror = (e) => {
  console.log(e)
};
