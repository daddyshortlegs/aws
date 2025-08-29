const http = require('http');
const WebSocket = require('ws');
const pty = require('node-pty');
const os = require('os');
const url = require('url');
const config = require('./config');

// Create HTTP server
const server = http.createServer((req, res) => {
  // Simple health check endpoint
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', timestamp: new Date().toISOString() }));
    return;
  }
  
  // Default response for other routes
  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('Not Found');
});

// Create WebSocket server
const wss = new WebSocket.Server({ server });

// WebSocket connection handler
wss.on('connection', (ws, req) => {
  if (config.logging.enableConnectionLogging) {
    console.log('Client connected from:', req.socket.remoteAddress);
  }
  
  // Parse query parameters from the WebSocket connection URL
  const parsedUrl = url.parse(req.url, true);
  const query = parsedUrl.query;
  
  // Extract SSH connection parameters from query string
  const sshHost = query.host || config.ssh.host;
  const sshPort = parseInt(query.port) || config.ssh.port;
  const sshUser = query.user || config.ssh.user;
  
  if (config.logging.enableConnectionLogging) {
    console.log(`SSH connection parameters: ${sshUser}@${sshHost}:${sshPort}`);
  }
  
  // Spawn SSH shell with dynamic parameters
  const shell = os.platform() === 'win32' ? 'powershell.exe' : 'bash';
  const ssh = pty.spawn('ssh', [
    '-p', sshPort.toString(),
    `${sshUser}@${sshHost}`
  ], {
    name: config.terminal.name,
    cols: config.terminal.cols,
    rows: config.terminal.rows,
    cwd: process.env.HOME,
    env: process.env,
  });

  // Forward data from SSH to WebSocket client
  ssh.on('data', (data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  });

  // Forward data from WebSocket client to SSH
  ws.on('message', (msg) => {
    try {
      const data = msg.toString();
      ssh.write(data);
    } catch (error) {
      console.error('Error writing to SSH:', error);
    }
  });

  // Handle WebSocket close
  ws.on('close', () => {
    console.log('Client disconnected');
    ssh.kill();
  });

  // Handle WebSocket errors
  ws.on('error', (error) => {
    console.error('WebSocket error:', error);
    ssh.kill();
  });

  // Handle SSH process exit
  ssh.on('exit', (code, signal) => {
    console.log(`SSH process exited with code ${code} and signal ${signal}`);
    if (ws.readyState === WebSocket.OPEN) {
      ws.close();
    }
  });
});

// Start server
server.listen(config.server.port, config.server.host, () => {
  console.log(`SSH WebSocket server running on ${config.server.host}:${config.server.port}`);
  console.log(`Health check: http://${config.server.host}:${config.server.port}/health`);
  console.log(`WebSocket endpoint: ws://${config.server.host}:${config.server.port}`);
  console.log(`Default SSH connection: ${config.ssh.user}@${config.ssh.host}:${config.ssh.port}`);
  console.log(`\nConnection examples:`);
  console.log(`  Default: ws://${config.server.host}:${config.server.port}`);
  console.log(`  Custom: ws://${config.server.host}:${config.server.port}?host=192.168.1.100&port=22&user=admin`);
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down server...');
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});

process.on('SIGTERM', () => {
  console.log('\nShutting down server...');
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});
