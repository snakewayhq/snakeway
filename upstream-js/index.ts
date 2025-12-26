import http from "http";
import {WebSocketServer} from "ws";

const server = http.createServer((req, res) => {
    res.writeHead(200);
    res.end("hello http\n");
});

const wss = new WebSocketServer({noServer: true});

wss.on("connection", (ws) => {
    ws.on("message", msg => ws.send(`echo: ${msg}`));
});

server.on("upgrade", (req, socket, head) => {
    wss.handleUpgrade(req, socket, head, ws => {
        wss.emit("connection", ws, req);
    });
});

server.listen(9000, () => {
    console.log("HTTP + WS upstream on :9000");
});
