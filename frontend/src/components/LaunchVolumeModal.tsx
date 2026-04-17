import React, { useState } from 'react';
import { getApiUrl } from '../config';

interface LaunchVolumeModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLaunch: (name: string) => void;
}

const modalStyle: React.CSSProperties = {
  display: 'block',
  zIndex: 1055,
  pointerEvents: 'auto',
};

const backdropStyle: React.CSSProperties = {
  zIndex: 1050,
};

const LaunchVolumeModal: React.FC<LaunchVolumeModalProps> = ({ isOpen, onClose, onLaunch }) => {
  const [volumeName, setVolumeName] = useState('');
  const [sizeGb, setSizeGb] = useState(10);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!volumeName.trim()) {
      setError('Volume name is required');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl('launchVolume'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: volumeName.trim(),
          size_gb: sizeGb,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();

      if (data.success) {
        onLaunch(volumeName.trim());
        handleClose();
      } else {
        setError(data.message || 'Failed to create volume');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create volume');
      console.error('Error creating volume:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    setVolumeName('');
    setSizeGb(10);
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
              <h5 className="modal-title">Create New Volume</h5>
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
                  <label htmlFor="volumeName" className="form-label">
                    Volume Name *
                  </label>
                  <input
                    type="text"
                    className="form-control"
                    id="volumeName"
                    value={volumeName}
                    onChange={(e) => setVolumeName(e.target.value)}
                    placeholder="Enter volume name"
                    required
                    disabled={isLoading}
                  />
                </div>

                <div className="mb-3">
                  <label htmlFor="sizeGb" className="form-label">
                    Size (GB)
                  </label>
                  <input
                    type="number"
                    className="form-control"
                    id="sizeGb"
                    value={sizeGb}
                    onChange={(e) => setSizeGb(parseInt(e.target.value, 10))}
                    min={1}
                    max={1000}
                    required
                    disabled={isLoading}
                  />
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
                  disabled={isLoading || !volumeName.trim()}
                >
                  {isLoading ? (
                    <>
                      <span className="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>
                      Creating...
                    </>
                  ) : (
                    'Create Volume'
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

export default LaunchVolumeModal;
