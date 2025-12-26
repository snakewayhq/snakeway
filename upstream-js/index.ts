import http from "http";
import express from "express";
import {WebSocketServer} from "ws";

const port = parseInt(process.argv[2] || "9000", 10);

const app = express();

app.get("/", (req, res) => {
    res.send("hello upstream\n");
});

app.get("/api/users/:id", (req, res) => {
    res.json({id: req.params.id});
})

const server = http.createServer(app);

const wss = new WebSocketServer({noServer: true});

wss.on("connection", (ws) => {
    ws.on("message", msg => ws.send(`echo: ${msg}`));
});

server.on("upgrade", (req, socket, head) => {
    wss.handleUpgrade(req, socket, head, ws => {
        wss.emit("connection", ws, req);
    });
});

server.listen(port, () => {
    console.log(`HTTP + WS upstream on :${port}`);
});
