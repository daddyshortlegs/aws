import React, { useState, useEffect } from 'react';
import { VM } from '../types';
import TerminalModal from './TerminalModal';

interface VMListProps {
  refreshKey?: number;
}

const VMList: React.FC<VMListProps> = ({ refreshKey = 0 }) => {
  const [vms, setVms] = useState<VM[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingVM, setDeletingVM] = useState<string | null>(null);
  const [terminalModal, setTerminalModal] = useState<{
    isOpen: boolean;
    vmName: string;
    sshPort: number;
  }>({
    isOpen: false,
    vmName: '',
    sshPort: 0,
  });

  useEffect(() => {
    fetchVMs();
  }, [refreshKey]);

  const fetchVMs = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await fetch('http://127.0.0.1:8080/list-vms', {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      setVms(data || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch VMs');
      console.error('Error fetching VMs:', err);
    } finally {
      setLoading(false);
    }
  };

  const deleteVM = async (vmId: string, vmName: string) => {
    if (!window.confirm(`Are you sure you want to delete VM "${vmName}"? This action cannot be undone.`)) {
      return;
    }

    try {
      setDeletingVM(vmId);
      setError(null);

      const response = await fetch('http://127.0.0.1:8080/delete-vm', {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          id: vmId,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result = await response.text();
      console.log('Delete result:', result);
      
      // Refresh the VM list after successful deletion
      await fetchVMs();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete VM');
      console.error('Error deleting VM:', err);
    } finally {
      setDeletingVM(null);
    }
  };

  const handleConnect = (vmName: string, sshPort: number) => {
    setTerminalModal({
      isOpen: true,
      vmName,
      sshPort,
    });
  };

  const closeTerminalModal = () => {
    setTerminalModal({
      isOpen: false,
      vmName: '',
      sshPort: 0,
    });
  };

  const getStatusBadgeClass = (pid: number) => {
    // For now, we'll assume if PID exists, the VM is running
    // In a real implementation, you'd check if the process is actually running
    return 'bg-success';
  };

  if (loading) {
    return (
      <div className="d-flex justify-content-center">
        <div className="spinner-border" role="status">
          <span className="visually-hidden">Loading...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="alert alert-danger" role="alert">
        <h4 className="alert-heading">Error loading VMs</h4>
        <p>{error}</p>
        <button className="btn btn-outline-danger" onClick={fetchVMs}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <>
      <div className="card">
        <div className="card-header d-flex justify-content-between align-items-center">
          <h5 className="card-title mb-0">Virtual Machines</h5>
          <button className="btn btn-primary btn-sm" onClick={fetchVMs}>
            <i className="bi bi-arrow-clockwise"></i> Refresh
          </button>
        </div>
        <div className="card-body">
          {vms.length === 0 ? (
            <div className="text-center py-4">
              <p className="text-muted">No VMs found</p>
              <button className="btn btn-primary">Create New VM</button>
            </div>
          ) : (
            <div className="table-responsive">
              <table className="table table-hover">
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>Status</th>
                    <th>SSH Port</th>
                    <th>PID</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {vms.map((vm) => (
                    <tr key={vm.id}>
                      <td>
                        <strong>{vm.name}</strong>
                        <br />
                        <small className="text-muted">ID: {vm.id}</small>
                      </td>
                      <td>
                        <span className={`badge ${getStatusBadgeClass(vm.pid)}`}>
                          Running
                        </span>
                      </td>
                      <td>
                        <code>{vm.ssh_port}</code>
                      </td>
                      <td>
                        <code>{vm.pid}</code>
                      </td>
                      <td>
                        <div className="btn-group" role="group">
                          <button 
                            className="btn btn-outline-primary btn-sm"
                            onClick={() => handleConnect(vm.name, vm.ssh_port)}
                          >
                            Connect
                          </button>
                          <button className="btn btn-outline-warning btn-sm">
                            Stop
                          </button>
                          <button 
                            className="btn btn-outline-danger btn-sm"
                            onClick={() => deleteVM(vm.id, vm.name)}
                            disabled={deletingVM === vm.id}
                          >
                            {deletingVM === vm.id ? (
                              <>
                                <span className="spinner-border spinner-border-sm me-1" role="status" aria-hidden="true"></span>
                                Deleting...
                              </>
                            ) : (
                              'Delete'
                            )}
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>

      <TerminalModal
        isOpen={terminalModal.isOpen}
        onClose={closeTerminalModal}
        vmName={terminalModal.vmName}
        sshPort={terminalModal.sshPort}
      />
    </>
  );
};

export default VMList; 