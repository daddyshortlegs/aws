module.exports = {
  // SSH connection configuration
  ssh: {
    host: process.env.SSH_HOST || 'localhost',
    port: process.env.SSH_PORT || 57757,
    user: process.env.SSH_USER || 'alpine',
    // Add more SSH options as needed
    // key: process.env.SSH_KEY_PATH,
    // password: process.env.SSH_PASSWORD,
  },

  // Server configuration
  server: {
    port: process.env.PORT || 3001,
    host: process.env.HOST || '0.0.0.0',
  },

  // Terminal configuration
  terminal: {
    cols: 120,
    rows: 30,
    name: 'xterm-color',
  },

  // Logging configuration
  logging: {
    level: process.env.LOG_LEVEL || 'info',
    enableConnectionLogging: true,
  }
};
