import React from 'react';
import './App.css';
import 'bootstrap/dist/css/bootstrap.min.css';
import VMList from './components/VMList';

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
            <div className="d-flex justify-content-between align-items-center mb-4">
              <h1>Virtual Machine Management</h1>
              <button className="btn btn-primary">
                <i className="bi bi-plus-circle"></i> Launch New VM
              </button>
            </div>
          </div>
        </div>

        <div className="row">
          <div className="col-12">
            <VMList />
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
