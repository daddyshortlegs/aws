# VM Orchestrator Frontend

This is a React TypeScript application with Bootstrap styling for the VM Orchestrator interface.

## Features

- React 18 with TypeScript
- Bootstrap 5 for responsive UI components
- Modern, clean interface design
- Responsive navigation
- Card-based layout for VM management

## Getting Started

### Prerequisites

- Node.js (version 16 or higher)
- npm or yarn

### Installation

1. Navigate to the frontend directory:
   ```bash
   cd frontend
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

### Development

To start the development server:

```bash
npm start
```

This will open the application in your browser at `http://localhost:3000`.

### Building for Production

To create a production build:

```bash
npm run build
```

The build files will be created in the `build` directory.

### Testing

To run tests:

```bash
npm test
```

## Project Structure

```
src/
├── App.tsx          # Main application component
├── App.css          # Custom styles
├── index.tsx        # Application entry point
└── ...
```

## Available Scripts

- `npm start` - Starts the development server
- `npm run build` - Creates a production build
- `npm test` - Runs the test suite
- `npm run eject` - Ejects from Create React App (one-way operation)

## Configuration

The frontend application can be configured to connect to different backend services. Configuration is handled through environment variables or by modifying the `src/config.ts` file.

### Environment Variables

Create a `.env` file in the frontend directory with the following variables:

```bash
# Backend service configuration
REACT_APP_BACKEND_HOST=127.0.0.1
REACT_APP_BACKEND_PORT=8080
REACT_APP_BACKEND_PROTOCOL=http
```

### Configuration File

The main configuration is in `src/config.ts`. You can modify this file to change:

- Backend host address
- Backend port number
- Protocol (http/https)
- API endpoints

### Quick Configuration Changes

To change the backend port from 8080 to another port (e.g., 3000):

1. **Option 1**: Set environment variable
   ```bash
   export REACT_APP_BACKEND_PORT=3000
   ```

2. **Option 2**: Modify `src/config.ts`
   ```typescript
   port: parseInt(process.env.REACT_APP_BACKEND_PORT || '3000', 10),
   ```

## Dependencies

- **React** - UI library
- **TypeScript** - Type safety
- **Bootstrap** - CSS framework for styling
- **@types/bootstrap** - TypeScript definitions for Bootstrap
