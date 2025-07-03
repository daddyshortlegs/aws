import React from 'react';
import './App.css';
import 'bootstrap/dist/css/bootstrap.min.css';

function App() {
  return (
    <div className="App">
      <nav className="navbar navbar-expand-lg navbar-dark bg-dark">
        <div className="container">
          <a className="navbar-brand" href="#home">VM Orchestrator</a>
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
            <div className="card">
              <div className="card-header">
                <h2 className="card-title mb-0">Welcome to VM Orchestrator</h2>
              </div>
              <div className="card-body">
                <p className="card-text">
                  This is a React TypeScript application with Bootstrap styling. 
                  You can use this as a starting point for your VM management interface.
                </p>
                <div className="d-grid gap-2 d-md-flex justify-content-md-start">
                  <button className="btn btn-primary me-md-2" type="button">
                    Launch VM
                  </button>
                  <button className="btn btn-outline-secondary" type="button">
                    View All VMs
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="row mt-4">
          <div className="col-md-6">
            <div className="card">
              <div className="card-header">
                <h5 className="card-title mb-0">Quick Actions</h5>
              </div>
              <div className="card-body">
                <div className="d-grid gap-2">
                  <button className="btn btn-success" type="button">
                    Create New VM
                  </button>
                  <button className="btn btn-info" type="button">
                    List Running VMs
                  </button>
                  <button className="btn btn-warning" type="button">
                    System Status
                  </button>
                </div>
              </div>
            </div>
          </div>
          <div className="col-md-6">
            <div className="card">
              <div className="card-header">
                <h5 className="card-title mb-0">System Information</h5>
              </div>
              <div className="card-body">
                <ul className="list-group list-group-flush">
                  <li className="list-group-item d-flex justify-content-between align-items-center">
                    Active VMs
                    <span className="badge bg-primary rounded-pill">0</span>
                  </li>
                  <li className="list-group-item d-flex justify-content-between align-items-center">
                    Total VMs
                    <span className="badge bg-secondary rounded-pill">0</span>
                  </li>
                  <li className="list-group-item d-flex justify-content-between align-items-center">
                    System Status
                    <span className="badge bg-success rounded-pill">Online</span>
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
