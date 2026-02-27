import React from 'react';
import SSHTerminal from './SSHTerminal';

interface TerminalModalProps {
  isOpen: boolean;
  onClose: () => void;
  vmName: string;
  sshPort: number;
}

const TerminalModal: React.FC<TerminalModalProps> = ({ isOpen, onClose, vmName, sshPort }) => {
  if (!isOpen) return null;

  return (
    <>
      <div className="modal fade show" style={{ display: 'block', zIndex: 1060 }} tabIndex={-1}>
        <div className="modal-dialog modal-xl" style={{ maxWidth: '90vw' }}>
          <div className="modal-content">
            <div className="modal-header">
              <h5 className="modal-title">
                <i className="bi bi-terminal"></i>
                SSH Terminal - {vmName}
              </h5>
              <button
                type="button"
                className="btn-close"
                onClick={onClose}
              ></button>
            </div>
            <div className="modal-body p-0">
              <SSHTerminal
                vmName={vmName}
                sshPort={sshPort}
                onClose={onClose}
              />
            </div>
          </div>
        </div>
      </div>
      <div className="modal-backdrop fade show" style={{ zIndex: 1055 }}></div>
    </>
  );
};

export default TerminalModal;
