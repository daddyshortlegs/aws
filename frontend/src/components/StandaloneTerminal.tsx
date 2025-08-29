import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

interface StandaloneTerminalProps {
  vmName: string;
  sshPort: number;
}

const StandaloneTerminal: React.FC<StandaloneTerminalProps> = ({ vmName, sshPort }) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminalInstance = useRef<Terminal | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    // Initialize terminal
    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#ffffff',
        cursor: '#ffffff',
      },
      cols: 120,
      rows: 30,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);

    terminal.open(terminalRef.current);
    
    // Fit terminal to container
    setTimeout(() => {
      fitAddon.fit();
    }, 100);

    // Handle window resize
    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener('resize', handleResize);

    // Create WebSocket connection with dynamic port
    const socket = new WebSocket(`ws://localhost:3001?port=${sshPort}`);
  
    socket.onopen = () => {
      setIsConnected(true);
      setConnectionError(null);
      terminal.writeln(`\r\n\x1b[32m✓ Connected to ${vmName} on port ${sshPort}\x1b[0m`);
      terminal.writeln('\r\n');
    };
  
    socket.onmessage = (event) => {
      terminal.write(event.data);
    };

    socket.onerror = (error) => {
      setConnectionError('WebSocket connection error');
      terminal.writeln(`\r\n\x1b[31m✗ Connection error: ${error}\x1b[0m\r\n`);
    };

    socket.onclose = () => {
      setIsConnected(false);
      terminal.writeln('\r\n\x1b[33m⚠ Connection closed\x1b[0m\r\n');
    };
  
    terminal.onData((data) => {
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(data);
      }
    });

    terminalInstance.current = terminal;

    return () => {
      window.removeEventListener('resize', handleResize);
      socket.close();
      terminal.dispose();
    };
  }, [vmName, sshPort]);

  const handleClose = () => {
    window.close();
  };

  return (
    <div className="standalone-terminal" style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Terminal Header */}
      <div className="terminal-header" style={{
        backgroundColor: '#2d2d2d',
        color: '#ffffff',
        padding: '10px 20px',
        borderBottom: '1px solid #444',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <div className="terminal-title">
          <i className="bi bi-terminal" style={{ marginRight: '8px' }}></i>
          SSH Terminal - {vmName} (Port: {sshPort})
        </div>
        <div className="terminal-controls">
          <span className={`connection-status ${isConnected ? 'connected' : 'disconnected'}`} style={{
            marginRight: '15px',
            fontSize: '12px',
            padding: '4px 8px',
            borderRadius: '4px',
            backgroundColor: isConnected ? '#28a745' : '#dc3545',
            color: 'white'
          }}>
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
          <button 
            className="btn btn-sm btn-outline-light" 
            onClick={handleClose}
            style={{ marginRight: '10px' }}
          >
            <i className="bi bi-x"></i> Close
          </button>
        </div>
      </div>

      {/* Terminal Content */}
      <div 
        ref={terminalRef} 
        className="terminal-content"
        style={{ 
          flex: 1,
          backgroundColor: '#1e1e1e',
          padding: '10px'
        }}
      />

      {/* Connection Error Display */}
      {connectionError && (
        <div className="connection-error" style={{
          backgroundColor: '#dc3545',
          color: 'white',
          padding: '10px 20px',
          textAlign: 'center',
          fontSize: '14px'
        }}>
          {connectionError}
        </div>
      )}
    </div>
  );
};

export default StandaloneTerminal;
