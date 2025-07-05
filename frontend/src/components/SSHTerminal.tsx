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

    // Write welcome message
    terminal.writeln(`\x1b[1;32mWelcome to SSH Terminal\x1b[0m`);
    terminal.writeln(`\x1b[1;36mConnecting to VM: ${vmName}\x1b[0m`);
    terminal.writeln(`\x1b[1;33mSSH Port: ${sshPort}\x1b[0m`);
    terminal.writeln('');

    // Simulate SSH connection (in a real implementation, you'd connect to a WebSocket proxy)
    terminal.writeln(`\x1b[1;34mEstablishing SSH connection...\x1b[0m`);
    
    setTimeout(() => {
      terminal.writeln(`\x1b[1;32m✓ Connected to ${vmName}\x1b[0m`);
      terminal.writeln(`\x1b[1;37mLast login: ${new Date().toLocaleString()}\x1b[0m`);
      terminal.writeln('');
      terminal.writeln(`\x1b[1;33m${vmName}@ubuntu:~$ \x1b[0m`);
    }, 1000);

    // Handle user input
    let currentLine = '';
    terminal.onData((data) => {
      if (data === '\r') {
        // Enter key pressed
        terminal.writeln('');
        handleCommand(currentLine, terminal);
        currentLine = '';
        terminal.write(`\x1b[1;33m${vmName}@ubuntu:~$ \x1b[0m`);
      } else if (data === '\u007F') {
        // Backspace
        if (currentLine.length > 0) {
          currentLine = currentLine.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (data >= ' ') {
        // Printable character
        currentLine += data;
        terminal.write(data);
      }
    });

    terminalInstance.current = terminal;

    return () => {
      window.removeEventListener('resize', handleResize);
      terminal.dispose();
    };
  }, [vmName, sshPort]);

  const handleCommand = (command: string, terminal: Terminal) => {
    const trimmedCommand = command.trim();
    
    if (!trimmedCommand) return;

    // Simulate some basic commands
    switch (trimmedCommand) {
      case 'ls':
        terminal.writeln('Desktop  Documents  Downloads  Pictures  Videos');
        break;
      case 'pwd':
        terminal.writeln('/home/ubuntu');
        break;
      case 'whoami':
        terminal.writeln('ubuntu');
        break;
      case 'date':
        terminal.writeln(new Date().toString());
        break;
      case 'exit':
        terminal.writeln('\x1b[1;31mConnection closed.\x1b[0m');
        setTimeout(() => onClose(), 1000);
        break;
      default:
        terminal.writeln(`\x1b[1;31mbash: ${trimmedCommand}: command not found\x1b[0m`);
    }
  };

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