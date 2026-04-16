import { io, Socket } from 'socket.io-client';
import type { WOSKernelDocument } from '../types/wos/kernel';
import type { IRealtimePort, Unsubscribe } from './WosPorts';

type KernelCallback = (kernel: WOSKernelDocument) => void;
type CollaboratorsCallback = (users: { id: string; name: string; cursor: { x: number; y: number } }[]) => void;
type CursorCallback = (cursors: { userId: string; cursor: { x: number; y: number } }) => void;

export class SocketIORealtimePort implements IRealtimePort {
  private socket: Socket | null = null;
  private kernelInitCbs: KernelCallback[] = [];
  private kernelChangedCbs: KernelCallback[] = [];
  private collaboratorsCbs: CollaboratorsCallback[] = [];
  private cursorCbs: CursorCallback[] = [];

  connect() {
    this.socket = io(window.location.origin);
    this.socket.on('kernel:init', (kernel: WOSKernelDocument) => { for (const cb of this.kernelInitCbs) cb(kernel); });
    this.socket.on('kernel:changed', (kernel: WOSKernelDocument) => { for (const cb of this.kernelChangedCbs) cb(kernel); });
    this.socket.on('users:update', (users: Parameters<CollaboratorsCallback>[0]) => { for (const cb of this.collaboratorsCbs) cb(users); });
    this.socket.on('cursor:update', (data: Parameters<CursorCallback>[0]) => { for (const cb of this.cursorCbs) cb(data); });
  }

  disconnect() {
    this.socket?.disconnect();
    this.socket = null;
  }

  onKernelInit(cb: KernelCallback): Unsubscribe {
    this.kernelInitCbs.push(cb);
    return () => {
      this.kernelInitCbs = this.kernelInitCbs.filter(c => c !== cb);
      if (this.kernelInitCbs.length === 0) this.socket?.off('kernel:init');
    };
  }

  onKernelChanged(cb: KernelCallback): Unsubscribe {
    this.kernelChangedCbs.push(cb);
    return () => {
      this.kernelChangedCbs = this.kernelChangedCbs.filter(c => c !== cb);
      if (this.kernelChangedCbs.length === 0) this.socket?.off('kernel:changed');
    };
  }

  onCollaboratorsUpdate(cb: CollaboratorsCallback): Unsubscribe {
    this.collaboratorsCbs.push(cb);
    return () => {
      this.collaboratorsCbs = this.collaboratorsCbs.filter(c => c !== cb);
      if (this.collaboratorsCbs.length === 0) this.socket?.off('users:update');
    };
  }

  onCursorUpdate(cb: CursorCallback): Unsubscribe {
    this.cursorCbs.push(cb);
    return () => {
      this.cursorCbs = this.cursorCbs.filter(c => c !== cb);
      if (this.cursorCbs.length === 0) this.socket?.off('cursor:update');
    };
  }

  sendCursorMove(pos: { x: number; y: number }) { this.socket?.emit('cursor:move', pos); }
  sendKernelUpdate(kernel: WOSKernelDocument) { this.socket?.emit('kernel:update', kernel); }
}
