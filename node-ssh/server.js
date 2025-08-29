const express = require('express');
const http = require('http');
const WebSocket = require('ws');
const pty = require('node-pty');
const os = require('os');

const app = express();
const server = http.createServer(app);
const wss = new WebSocket.Server({ server });

app.use(express.static(__dirname + '/public')); // serve your HTML

wss.on('connection', (ws) => {
  console.log('Client connected');

  // 🟢 Spawn SSH shell (modify user@host to your target VM)
  const shell = os.platform() === 'win32' ? 'powershell.exe' : 'bash';
  const ssh = pty.spawn('ssh', ['-p 62388', 'andy@localhost'], {
    name: 'xterm-color',
    cols: 80,
    rows: 30,
    cwd: process.env.HOME,
    env: process.env,
  });

  ssh.on('data', (data) => {
    ws.send(data);
  });

  ws.on('message', (msg) => {
    ssh.write(msg);
  });

  ws.on('close', () => {
    ssh.kill();
    console.log('Client disconnected');
  });
});

server.listen(3001, () => {
  console.log('Server running on http://localhost:3001');
});
