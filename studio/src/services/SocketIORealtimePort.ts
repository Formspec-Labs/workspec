import { io, Socket } from 'socket.io-client';
import type { WOSKernelDocument } from '../types/wos/kernel';
import type { IRealtimePort } from './WosPorts';

export class SocketIORealtimePort implements IRealtimePort {
  private socket: Socket | null = null;
  private kernelInitCb: ((kernel: WOSKernelDocument) => void) | null = null;
  private kernelChangedCb: ((kernel: WOSKernelDocument) => void) | null = null;
  private collaboratorsCb: ((users: { id: string; name: string; cursor: { x: number; y: number } }[]) => void) | null = null;
  private cursorCb: ((cursors: { userId: string; cursor: { x: number; y: number } }) => void) | null = null;

  connect() {
    this.socket = io(window.location.origin);
    this.socket.on('kernel:init', (kernel: WOSKernelDocument) => this.kernelInitCb?.(kernel));
    this.socket.on('kernel:changed', (kernel: WOSKernelDocument) => this.kernelChangedCb?.(kernel));
    this.socket.on('users:update', (users: { id: string; name: string; cursor: { x: number; y: number } }[]) => this.collaboratorsCb?.(users));
    this.socket.on('cursor:update', (data: { userId: string; cursor: { x: number; y: number } }) => this.cursorCb?.(data));
  }

  disconnect() {
    this.socket?.disconnect();
    this.socket = null;
  }

  onKernelInit(cb: (kernel: WOSKernelDocument) => void) { this.kernelInitCb = cb; }
  onKernelChanged(cb: (kernel: WOSKernelDocument) => void) { this.kernelChangedCb = cb; }
  onCollaboratorsUpdate(cb: (users: { id: string; name: string; cursor: { x: number; y: number } }[]) => void) { this.collaboratorsCb = cb; }
  onCursorUpdate(cb: (cursors: { userId: string; cursor: { x: number; y: number } }) => void) { this.cursorCb = cb; }

  sendCursorMove(pos: { x: number; y: number }) { this.socket?.emit('cursor:move', pos); }
  sendKernelUpdate(kernel: WOSKernelDocument) { this.socket?.emit('kernel:update', kernel); }
}
