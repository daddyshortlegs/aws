import React, { useState } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import './App.css';
import 'bootstrap/dist/css/bootstrap.min.css';
import VMList from './components/VMList';
import VolumeList from './components/VolumeList';
import LaunchVMModal from './components/LaunchVMModal';
import LaunchVolumeModal from './components/LaunchVolumeModal';
import TerminalPage from './pages/TerminalPage';

type Tab = 'vms' | 'volumes';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('vms');
  const [isVMModalOpen, setIsVMModalOpen] = useState(false);
  const [isVolumeModalOpen, setIsVolumeModalOpen] = useState(false);
  const [vmRefreshKey, setVmRefreshKey] = useState(0);
  const [volumeRefreshKey, setVolumeRefreshKey] = useState(0);

  return (
    <Router>
      <div className="App">
        <Routes>
          <Route path="/terminal" element={<TerminalPage />} />

          <Route path="/*" element={
            <>
              <nav className="navbar navbar-expand-lg navbar-dark bg-dark">
                <div className="container-fluid">
                  <span className="navbar-brand">Andy's Web Services</span>
                </div>
              </nav>

              <div className="container-fluid mt-4">
                <div className="row">
                  <div className="col-auto">
                    <div className="nav flex-column nav-pills">
                      <button
                        className={`nav-link text-start ${activeTab === 'vms' ? 'active' : ''}`}
                        onClick={() => setActiveTab('vms')}
                      >
                        VMs
                      </button>
                      <button
                        className={`nav-link text-start ${activeTab === 'volumes' ? 'active' : ''}`}
                        onClick={() => setActiveTab('volumes')}
                      >
                        Volumes
                      </button>
                    </div>
                  </div>

                  <div className="col">
                    {activeTab === 'vms' && (
                      <>
                        <div className="d-flex justify-content-between align-items-center mb-4">
                          <h1>Virtual Machine Management</h1>
                          <button
                            className="btn btn-primary"
                            onClick={() => setIsVMModalOpen(true)}
                          >
                            <i className="bi bi-plus-circle"></i> Launch New VM
                          </button>
                        </div>
                        <VMList refreshKey={vmRefreshKey} />
                      </>
                    )}

                    {activeTab === 'volumes' && (
                      <>
                        <div className="d-flex justify-content-between align-items-center mb-4">
                          <h1>Volumes</h1>
                          <button
                            className="btn btn-primary"
                            onClick={() => setIsVolumeModalOpen(true)}
                          >
                            <i className="bi bi-plus-circle"></i> Create New Volume
                          </button>
                        </div>
                        <VolumeList refreshKey={volumeRefreshKey} />
                      </>
                    )}
                  </div>
                </div>
              </div>

              <LaunchVMModal
                isOpen={isVMModalOpen}
                onClose={() => setIsVMModalOpen(false)}
                onLaunch={() => setVmRefreshKey(prev => prev + 1)}
              />
              <LaunchVolumeModal
                isOpen={isVolumeModalOpen}
                onClose={() => setIsVolumeModalOpen(false)}
                onLaunch={() => setVolumeRefreshKey(prev => prev + 1)}
              />
            </>
          } />
        </Routes>
      </div>
    </Router>
  );
}

export default App;
