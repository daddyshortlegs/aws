This directory contains a backend websocket application to handle input and ouput
from a webapp that uses xterm.js to simulate an SSH connection. The backend
uses node-pty to handle psuedo terminals to allow SSH connections to be established
to an SSH server.

To run:

```
npm run start
```

Open a browser at http://localhost:3001.

The front-end will open a websocket to the same URL.

