import React, { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

interface SSHTerminalProps {
  vmName: string;
  sshPort: number;
  onClose: () => void;
}

const SSHTerminal: React.FC<SSHTerminalProps> = ({ vmName, sshPort, onClose }) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminalInstance = useRef<Terminal | null>(null);

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
      cols: 80,
      rows: 24,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);

    terminal.open(terminalRef.current);
    // Delay fit to allow modal to render and have non-zero dimensions
    setTimeout(() => {
      fitAddon.fit();
    }, 0);

    // Handle window resize
    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener('resize', handleResize);


    const socket = new WebSocket(`ws://localhost:3001?port=${sshPort}`);

    socket.onopen = () => {
      terminal.writeln('Connected to server.');
    };

    socket.onmessage = (event) => {
      terminal.write(event.data);
    };

    terminal.onData((data) => {
      socket.send(data);
    });

    terminalInstance.current = terminal;

    return () => {
      window.removeEventListener('resize', handleResize);
      terminal.dispose();
    };
  }, [vmName, sshPort]);



  return (
    <div className="ssh-terminal-container">
      <div className="terminal-header">
        <div className="terminal-title">
          <i className="bi bi-terminal"></i>
          SSH Terminal - {vmName} (Port: {sshPort})
        </div>
        <button className="btn btn-sm btn-outline-secondary" onClick={onClose}>
          <i className="bi bi-x"></i>
        </button>
      </div>
      <div
        ref={terminalRef}
        className="terminal-content"
        style={{
          height: '400px',
          backgroundColor: '#1e1e1e',
          padding: '10px'
        }}
      />
    </div>
  );
};

export default SSHTerminal;
