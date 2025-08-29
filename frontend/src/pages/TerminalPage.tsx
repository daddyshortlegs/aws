import React, { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import StandaloneTerminal from '../components/StandaloneTerminal';

const TerminalPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const [vmName, setVmName] = useState<string>('');
  const [sshPort, setSshPort] = useState<number>(0);
  const [isValid, setIsValid] = useState<boolean>(false);

  useEffect(() => {
    // Extract parameters from URL
    const vm = searchParams.get('vm');
    const port = searchParams.get('port');

    if (vm && port) {
      const portNum = parseInt(port, 10);
      if (portNum > 0) {
        setVmName(vm);
        setSshPort(portNum);
        setIsValid(true);
      } else {
        setIsValid(false);
      }
    } else {
      setIsValid(false);
    }
  }, [searchParams]);

  if (!isValid) {
    return (
      <div style={{
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: '#f8f9fa',
        fontFamily: 'Arial, sans-serif'
      }}>
        <div style={{
          backgroundColor: 'white',
          padding: '40px',
          borderRadius: '8px',
          boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
          textAlign: 'center',
          maxWidth: '500px'
        }}>
          <div style={{ fontSize: '48px', marginBottom: '20px' }}>⚠️</div>
          <h2 style={{ color: '#dc3545', marginBottom: '20px' }}>Invalid Terminal Parameters</h2>
          <p style={{ color: '#6c757d', marginBottom: '20px' }}>
            The terminal URL is missing required parameters or contains invalid values.
          </p>
          <div style={{ 
            backgroundColor: '#f8f9fa', 
            padding: '15px', 
            borderRadius: '4px',
            fontFamily: 'monospace',
            fontSize: '14px',
            textAlign: 'left'
          }}>
            <div>Required format:</div>
            <div>/terminal?vm=VM_NAME&port=SSH_PORT</div>
            <br />
            <div>Example:</div>
            <div>/terminal?vm=my-vm&port=22</div>
          </div>
          <button 
            onClick={() => window.close()}
            style={{
              marginTop: '20px',
              padding: '10px 20px',
              backgroundColor: '#6c757d',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Close Window
          </button>
        </div>
      </div>
    );
  }

  return <StandaloneTerminal vmName={vmName} sshPort={sshPort} />;
};

export default TerminalPage;
