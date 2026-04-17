import { io, Socket } from 'socket.io-client';
import type { WOSKernelDocument } from '../types/wos/kernel';
import type { IRealtimePort, Unsubscribe } from './WosPorts';
import { getAccessToken } from './authedFetch';

type KernelCallback = (kernel: WOSKernelDocument, url?: string) => void;
type CollaboratorsCallback = (users: { id: string; name: string; cursor: { x: number; y: number } }[]) => void;
type CursorCallback = (cursors: { userId: string; cursor: { x: number; y: number } }) => void;

interface KernelEnvelope {
  url?: string;
  kernel: WOSKernelDocument;
}

function isEnvelope(value: unknown): value is KernelEnvelope {
  return !!value && typeof value === 'object' && 'kernel' in value;
}

function dispatchKernel(value: unknown, listeners: KernelCallback[]) {
  if (isEnvelope(value)) {
    for (const cb of listeners) cb(value.kernel, value.url);
  } else if (value && typeof value === 'object') {
    for (const cb of listeners) cb(value as WOSKernelDocument);
  }
}

export class SocketIORealtimePort implements IRealtimePort {
  private socket: Socket | null = null;
  private kernelInitCbs: KernelCallback[] = [];
  private kernelChangedCbs: KernelCallback[] = [];
  private collaboratorsCbs: CollaboratorsCallback[] = [];
  private cursorCbs: CursorCallback[] = [];
  private listenersAttached = false;

  connect() {
    if (this.socket) return;
    const token = getAccessToken();
    this.socket = io(window.location.origin, token ? { auth: { token } } : {});
    if (!this.listenersAttached) {
      this.socket.on('kernel:init', (value: unknown) => dispatchKernel(value, this.kernelInitCbs));
      this.socket.on('kernel:changed', (value: unknown) => dispatchKernel(value, this.kernelChangedCbs));
      this.socket.on('users:update', (users: Parameters<CollaboratorsCallback>[0]) => {
        for (const cb of this.collaboratorsCbs) cb(users);
      });
      this.socket.on('cursor:update', (data: Parameters<CursorCallback>[0]) => {
        for (const cb of this.cursorCbs) cb(data);
      });
      this.listenersAttached = true;
    }
  }

  disconnect() {
    this.socket?.disconnect();
    this.socket = null;
    this.listenersAttached = false;
    this.kernelInitCbs = [];
    this.kernelChangedCbs = [];
    this.collaboratorsCbs = [];
    this.cursorCbs = [];
  }

  onKernelInit(cb: KernelCallback): Unsubscribe {
    this.kernelInitCbs.push(cb);
    return () => { this.kernelInitCbs = this.kernelInitCbs.filter(c => c !== cb); };
  }

  onKernelChanged(cb: KernelCallback): Unsubscribe {
    this.kernelChangedCbs.push(cb);
    return () => { this.kernelChangedCbs = this.kernelChangedCbs.filter(c => c !== cb); };
  }

  onCollaboratorsUpdate(cb: CollaboratorsCallback): Unsubscribe {
    this.collaboratorsCbs.push(cb);
    return () => { this.collaboratorsCbs = this.collaboratorsCbs.filter(c => c !== cb); };
  }

  onCursorUpdate(cb: CursorCallback): Unsubscribe {
    this.cursorCbs.push(cb);
    return () => { this.cursorCbs = this.cursorCbs.filter(c => c !== cb); };
  }

  sendCursorMove(pos: { x: number; y: number }) {
    this.socket?.emit('cursor:move', pos);
  }

  sendKernelUpdate(kernel: WOSKernelDocument, url?: string) {
    const targetUrl = url ?? kernel.url;
    if (!targetUrl) return;
    this.socket?.emit('kernel:update', { url: targetUrl, kernel });
  }
}
