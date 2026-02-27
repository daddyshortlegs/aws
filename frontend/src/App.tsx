import React, { useState } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import './App.css';
import 'bootstrap/dist/css/bootstrap.min.css';
import VMList from './components/VMList';
import LaunchVMModal from './components/LaunchVMModal';
import TerminalPage from './pages/TerminalPage';

function App() {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  const handleLaunchVM = (name: string, instanceType: string) => {
    // The modal will handle the API call, this is just for any additional actions
    console.log(`VM launched: ${name} with type ${instanceType}`);
    // Trigger a refresh of the VM list
    setRefreshKey(prev => prev + 1);
  };

  return (
    <Router>
      <div className="App">
        <Routes>
          {/* Terminal route - full screen terminal */}
          <Route path="/terminal" element={<TerminalPage />} />

          {/* Main application route */}
          <Route path="/*" element={
            <>
              <nav className="navbar navbar-expand-lg navbar-dark bg-dark">
                <div className="container">
                  <a className="navbar-brand" href="#home">Andy's Web Services</a>
                  <button className="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarNav" aria-controls="navbarNav" aria-expanded="false" aria-label="Toggle navigation">
                    <span className="navbar-toggler-icon"></span>
                  </button>
                  <div className="collapse navbar-collapse" id="navbarNav">
                    <ul className="navbar-nav ms-auto">
                      <li className="nav-item">
                        <a className="nav-link active" aria-current="page" href="#home">Home</a>
                      </li>
                      <li className="nav-item">
                        <a className="nav-link" href="#vms">VMs</a>
                      </li>
                      <li className="nav-item">
                        <a className="nav-link" href="#settings">Settings</a>
                      </li>
                    </ul>
                  </div>
                </div>
              </nav>

              <main className="container mt-4">
                <div className="row">
                  <div className="col-12">
                    <div className="d-flex justify-content-between align-items-center mb-4">
                      <h1>Virtual Machine Management</h1>
                      <button
                        className="btn btn-primary"
                        onClick={() => setIsModalOpen(true)}
                      >
                        <i className="bi bi-plus-circle"></i> Launch New VM
                      </button>
                    </div>
                  </div>
                </div>

                <div className="row">
                  <div className="col-12">
                    <VMList refreshKey={refreshKey} />
                  </div>
                </div>
              </main>

              <LaunchVMModal
                isOpen={isModalOpen}
                onClose={() => setIsModalOpen(false)}
                onLaunch={handleLaunchVM}
              />
            </>
          } />
        </Routes>
      </div>
    </Router>
  );
}

export default App;
