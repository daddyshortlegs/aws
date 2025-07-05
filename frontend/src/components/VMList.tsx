import React, { useState, useEffect } from 'react';
import { VM } from '../types';

interface VMListProps {
  refreshKey?: number;
}

const VMList: React.FC<VMListProps> = ({ refreshKey = 0 }) => {
  const [vms, setVms] = useState<VM[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
                        <button className="btn btn-outline-primary btn-sm">
                          Connect
                        </button>
                        <button className="btn btn-outline-warning btn-sm">
                          Stop
                        </button>
                        <button className="btn btn-outline-danger btn-sm">
                          Delete
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
  );
};

export default VMList; 