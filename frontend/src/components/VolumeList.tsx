import React, { useState, useEffect } from 'react';
import { Volume } from '../types';
import { getApiUrl } from '../config';

interface VolumeListProps {
  refreshKey?: number;
}

const VolumeList: React.FC<VolumeListProps> = ({ refreshKey = 0 }) => {
  const [volumes, setVolumes] = useState<Volume[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingVolume, setDeletingVolume] = useState<string | null>(null);

  useEffect(() => {
    fetchVolumes();
  }, [refreshKey]);

  const fetchVolumes = async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await fetch(getApiUrl('listVolumes'), {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      setVolumes(data || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch volumes');
      console.error('Error fetching volumes:', err);
    } finally {
      setLoading(false);
    }
  };

  const deleteVolume = async (volumeId: string, volumeName: string) => {
    if (!window.confirm(`Are you sure you want to delete volume "${volumeName}"? This action cannot be undone.`)) {
      return;
    }

    try {
      setDeletingVolume(volumeId);
      setError(null);

      const response = await fetch(getApiUrl('deleteVolume'), {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ id: volumeId }),
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `HTTP error! status: ${response.status}`);
      }

      await fetchVolumes();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete volume');
      console.error('Error deleting volume:', err);
    } finally {
      setDeletingVolume(null);
    }
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
        <h4 className="alert-heading">Error loading volumes</h4>
        <p>{error}</p>
        <button className="btn btn-outline-danger" onClick={fetchVolumes}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="card-header d-flex justify-content-between align-items-center">
        <h5 className="card-title mb-0">Volumes</h5>
        <button className="btn btn-primary btn-sm" onClick={fetchVolumes}>
          <i className="bi bi-arrow-clockwise"></i> Refresh
        </button>
      </div>
      <div className="card-body">
        {volumes.length === 0 ? (
          <div className="text-center py-4">
            <p className="text-muted">No volumes found</p>
          </div>
        ) : (
          <div className="table-responsive">
            <table className="table table-hover">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Mount Path</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {volumes.map((volume) => (
                  <tr key={volume.id}>
                    <td>
                      <strong>{volume.name}</strong>
                      <br />
                      <small className="text-muted">ID: {volume.id}</small>
                    </td>
                    <td>
                      <code>{volume.mount_path}</code>
                    </td>
                    <td>
                      <button
                        className="btn btn-outline-danger btn-sm"
                        onClick={() => deleteVolume(volume.id, volume.name)}
                        disabled={deletingVolume === volume.id}
                      >
                        {deletingVolume === volume.id ? (
                          <>
                            <span className="spinner-border spinner-border-sm me-1" role="status" aria-hidden="true"></span>
                            Deleting...
                          </>
                        ) : (
                          'Delete'
                        )}
                      </button>
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

export default VolumeList;
