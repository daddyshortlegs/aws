import React, { useState } from 'react';
import { getApiUrl } from '../config';

interface LaunchVMModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLaunch: (name: string, instanceType: string) => void;
}

const modalStyle: React.CSSProperties = {
  display: 'block',
  zIndex: 1055,
  pointerEvents: 'auto',
};

const backdropStyle: React.CSSProperties = {
  zIndex: 1050,
};

const LaunchVMModal: React.FC<LaunchVMModalProps> = ({ isOpen, onClose, onLaunch }) => {
  const [vmName, setVmName] = useState('');
  const [instanceType, setInstanceType] = useState('t2.micro');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const instanceTypes = [
    { value: 't2.micro', label: 't2.micro - 1 vCPU, 1 GB RAM' },
    { value: 't2.small', label: 't2.small - 1 vCPU, 2 GB RAM' },
    { value: 't2.medium', label: 't2.medium - 2 vCPU, 4 GB RAM' },
    { value: 't2.large', label: 't2.large - 2 vCPU, 8 GB RAM' },
    { value: 't2.xlarge', label: 't2.xlarge - 4 vCPU, 16 GB RAM' },
  ];

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!vmName.trim()) {
      setError('VM name is required');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl('launchVM'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: vmName.trim(),
          instance_type: instanceType,
          region: 'us-east-1', // Default region
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      
      if (data.success) {
        onLaunch(vmName.trim(), instanceType);
        handleClose();
      } else {
        setError(data.message || 'Failed to launch VM');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to launch VM');
      console.error('Error launching VM:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    setVmName('');
    setInstanceType('t2.micro');
    setError(null);
    setIsLoading(false);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <>
      <div className="modal fade show" style={modalStyle} tabIndex={-1}>
        <div className="modal-dialog">
          <div className="modal-content">
            <div className="modal-header">
              <h5 className="modal-title">Launch New Virtual Machine</h5>
              <button
                type="button"
                className="btn-close"
                onClick={handleClose}
                disabled={isLoading}
              ></button>
            </div>
            <form onSubmit={handleSubmit}>
              <div className="modal-body">
                {error && (
                  <div className="alert alert-danger" role="alert">
                    {error}
                  </div>
                )}
                
                <div className="mb-3">
                  <label htmlFor="vmName" className="form-label">
                    VM Name *
                  </label>
                  <input
                    type="text"
                    className="form-control"
                    id="vmName"
                    value={vmName}
                    onChange={(e) => setVmName(e.target.value)}
                    placeholder="Enter VM name"
                    required
                    disabled={isLoading}
                  />
                </div>

                <div className="mb-3">
                  <label htmlFor="instanceType" className="form-label">
                    Instance Type
                  </label>
                  <select
                    className="form-select"
                    id="instanceType"
                    value={instanceType}
                    onChange={(e) => setInstanceType(e.target.value)}
                    disabled={isLoading}
                  >
                    {instanceTypes.map((type) => (
                      <option key={type.value} value={type.value}>
                        {type.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="mb-3">
                  <label className="form-label">Region</label>
                  <input
                    type="text"
                    className="form-control"
                    value="us-east-1"
                    disabled
                  />
                  <div className="form-text">
                    Currently using default region. Region selection will be available in future updates.
                  </div>
                </div>
              </div>
              <div className="modal-footer">
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={handleClose}
                  disabled={isLoading}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={isLoading || !vmName.trim()}
                >
                  {isLoading ? (
                    <>
                      <span className="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>
                      Launching...
                    </>
                  ) : (
                    'Launch VM'
                  )}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
      <div className="modal-backdrop fade show" style={backdropStyle}></div>
    </>
  );
};

export default LaunchVMModal; 