// Frontend configuration
export const config = {
  // Backend service configuration
  backend: {
    host: process.env.REACT_APP_BACKEND_HOST || '127.0.0.1',
    port: parseInt(process.env.REACT_APP_BACKEND_PORT || '8080', 10),
    protocol: process.env.REACT_APP_BACKEND_PROTOCOL || 'http',
  },

  // Get the full backend URL.
  // In production (no REACT_APP_BACKEND_HOST set) use a relative /api path so
  // requests stay on the same origin and are proxied by nginx — no CORS needed.
  // In local dev set REACT_APP_BACKEND_HOST=127.0.0.1 to hit the proxy directly.
  getBackendUrl: (endpoint: string = '') => {
    if (process.env.REACT_APP_BACKEND_HOST) {
      const { protocol, host, port } = config.backend;
      return `${protocol}://${host}:${port}${endpoint}`;
    }
    return `/api${endpoint}`;
  },

  // Returns the base WebSocket URL for the node-ssh service.
  // In production (no REACT_APP_BACKEND_HOST) use a relative /ssh path proxied
  // by nginx. In local dev hit node-ssh directly on port 3001.
  getSshWsUrl: () => {
    if (process.env.REACT_APP_BACKEND_HOST) {
      return 'ws://localhost:3001';
    }
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${wsProtocol}//${window.location.host}/ssh`;
  },

  // Common API endpoints
  endpoints: {
    launchVM: '/launch-vm',
    listVMs: '/list-vms',
    deleteVM: '/delete-vm',
    launchVolume: '/launch-volume',
    listVolumes: '/list-volumes',
    deleteVolume: '/delete-volume',
    websocket: '/ws',
  },
} as const;

// Type for the config
export type Config = typeof config;

// Helper function to get a specific endpoint URL
export const getApiUrl = (endpoint: keyof typeof config.endpoints) => {
  return config.getBackendUrl(config.endpoints[endpoint]);
};
