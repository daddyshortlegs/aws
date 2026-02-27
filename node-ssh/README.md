# SSH WebSocket Server

A simple Node.js WebSocket server that provides SSH terminal access through WebSocket connections.

## Installation

1. Install dependencies:
   ```bash
   npm install
   ```

2. Configure SSH connection parameters (see Configuration section)

3. Start the server:
   ```bash
   npm start
   ```

## Configuration

### Environment Variables

You can configure the server using environment variables:

```bash
# SSH connection
export SSH_HOST=your-vm-ip
export SSH_PORT=22
export SSH_USER=your-username

# Server configuration
export PORT=3001
export HOST=0.0.0.0

# Logging
export LOG_LEVEL=info
```

### Configuration File

The `config.js` file contains default values and can be modified directly:

```javascript
module.exports = {
  ssh: {
    host: 'localhost',
    port: 62388,
    user: 'andy',
  },
  server: {
    port: 3001,
    host: '0.0.0.0',
  },
  // ... more options
};
```

## Usage

### Starting the Server

```bash
# Development mode with auto-restart
npm run dev

# Production mode
npm start

# Custom port
PORT=8080 npm start
```

### Health Check

The server provides a health check endpoint:

```bash
curl http://localhost:3001/health
```

Response:
```json
{
  "status": "ok",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

## API Endpoints

- **WebSocket**: `ws://localhost:3001` - SSH terminal connection
- **Health Check**: `GET /health` - Server health status
- **Default**: `*` - Returns 404 Not Found

## WebSocket Events

### Server to Client
- **data**: Terminal output from SSH session

### Client to Server
- **message**: User input to send to SSH session

### Connection Events
- **connection**: New WebSocket client connected
- **close**: Client disconnected
- **error**: WebSocket or SSH error occurred

## Security Considerations

- The server runs SSH connections with the current user's permissions
- Consider implementing authentication for WebSocket connections
- SSH keys should be properly configured for secure connections
- The server binds to all interfaces by default (configurable via HOST env var)

## Troubleshooting

### Common Issues

1. **SSH Connection Failed**
   - Verify SSH credentials and server accessibility
   - Check SSH key permissions
   - Ensure target server allows SSH connections

2. **Permission Denied**
   - Verify the user has permission to spawn SSH processes
   - Check file permissions for SSH keys

3. **Port Already in Use**
   - Change the PORT environment variable
   - Kill existing processes using the port

### Debug Mode

Enable verbose logging by setting:

```bash
export LOG_LEVEL=debug
```

## Development

### File Structure

```
node-ssh/
├── server.js          # Main server file
├── config.js          # Configuration file
├── package.json       # Dependencies and scripts
└── README.md          # This file
```

### Adding Features

The modular structure makes it easy to add features:

1. **Authentication**: Add middleware to the WebSocket connection handler
2. **Multiple Sessions**: Track multiple SSH connections per client
3. **Terminal Resizing**: Handle terminal size changes
4. **File Transfer**: Add SCP/SFTP capabilities

## License

ISC
