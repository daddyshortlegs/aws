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

## Dependencies

- **React** - UI library
- **TypeScript** - Type safety
- **Bootstrap** - CSS framework for styling
- **@types/bootstrap** - TypeScript definitions for Bootstrap
