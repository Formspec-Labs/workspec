import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import { createServer as createViteServer } from 'vite';
import path from 'path';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function startServer() {
  const app = express();
  const httpServer = createServer(app);
  const io = new Server(httpServer, {
    cors: { origin: '*', methods: ['GET', 'POST'] },
  });

  const PORT = 3000;

  const kernelPath = path.resolve(__dirname, '../fixtures/kernel/benefits-adjudication.json');
  let kernelState = JSON.parse(readFileSync(kernelPath, 'utf-8'));

  app.use(express.json());

  app.get('/api/kernel', (_req, res) => {
    res.json(kernelState);
  });

  app.put('/api/kernel', (req, res) => {
    kernelState = req.body;
    io.emit('kernel:changed', kernelState);
    res.json({ ok: true });
  });

  const activeUsers = new Map();

  io.on('connection', (socket) => {
    console.log('User connected:', socket.id);

    socket.emit('kernel:init', kernelState);

    socket.on('user:join', (userData) => {
      activeUsers.set(socket.id, { ...userData, id: socket.id, cursor: { x: 0, y: 0 } });
      io.emit('users:update', Array.from(activeUsers.values()));
    });

    socket.on('cursor:move', (pos) => {
      const user = activeUsers.get(socket.id);
      if (user) {
        user.cursor = pos;
        socket.broadcast.emit('cursor:update', { userId: socket.id, cursor: pos });
      }
    });

    socket.on('kernel:update', (newKernel) => {
      kernelState = newKernel;
      socket.broadcast.emit('kernel:changed', kernelState);
    });

    socket.on('disconnect', () => {
      activeUsers.delete(socket.id);
      io.emit('users:update', Array.from(activeUsers.values()));
      console.log('User disconnected:', socket.id);
    });
  });

  if (process.env.NODE_ENV !== 'production') {
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: 'spa',
    });
    app.use(vite.middlewares);
  } else {
    const distPath = path.join(process.cwd(), 'dist');
    app.use(express.static(distPath));
    app.get('*', (req, res) => {
      res.sendFile(path.join(distPath, 'index.html'));
    });
  }

  httpServer.listen(PORT, '0.0.0.0', () => {
    console.log(`Server running on http://localhost:${PORT}`);
    console.log(`Kernel loaded: ${kernelState.title} v${kernelState.version}`);
  });
}

startServer();
