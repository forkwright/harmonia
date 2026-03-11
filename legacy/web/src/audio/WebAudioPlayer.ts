// Pure HTMLAudioElement player — no Web Audio API processing
// The browser's native decoder is the cleanest signal path.
import type { Track } from '../types';
import type { EqualizerProcessor } from './EqualizerProcessor';
import { logError, logInfo } from '../utils/errorLogger';

export interface AudioPipelineState {
  inputFormat: { sampleRate: number; bitDepth: number; channels: number; codec: string };
  outputDevice: { sampleRate: number; channels: number };
  bufferSize: number;
  latency: number;
}

export class WebAudioPlayer {
  private audioElement: HTMLAudioElement | null = null;
  private isPlaying = false;
  private volume = 1;
  private playbackSpeed = 1;

  // Preload
  private nextAudioElement: HTMLAudioElement | null = null;
  private nextTrack: Track | null = null;

  // Callbacks
  private onPlaybackEnd?: () => void;
  private onPlaybackError?: (error: Error) => void;

  private buildStreamUrl(streamUrl: string): string {
    const token = localStorage.getItem('accessToken');
    if (!token) return streamUrl;
    const separator = streamUrl.includes('?') ? '&' : '?';
    return `${streamUrl}${separator}token=${encodeURIComponent(token)}`;
  }

  private createAudioElement(streamUrl: string): HTMLAudioElement {
    const audio = new Audio();
    audio.preload = 'auto';
    audio.volume = this.volume;
    audio.playbackRate = this.playbackSpeed;
    audio.src = this.buildStreamUrl(streamUrl);
    return audio;
  }

  async loadTrack(track: Track, streamUrl: string): Promise<void> {
    this.stopCurrent();

    try {
      if (this.nextTrack?.id === track.id && this.nextAudioElement) {
        this.audioElement = this.nextAudioElement;
        this.nextAudioElement = null;
        this.nextTrack = null;
      } else {
        this.audioElement = this.createAudioElement(streamUrl);
      }

      this.audioElement.volume = this.volume;
      this.audioElement.playbackRate = this.playbackSpeed;

      // Store streamUrl for potential retry after token refresh
      const retryUrl = streamUrl;

      this.audioElement.onended = () => {
        this.isPlaying = false;
        this.onPlaybackEnd?.();
      };

      this.audioElement.onerror = () => {
        const code = this.audioElement?.error?.code;
        const msg = this.audioElement?.error?.message || `Audio error code ${code}`;

        // Error code 4 = MEDIA_ERR_SRC_NOT_SUPPORTED — often a 401 returning JSON
        // Error code 2 = MEDIA_ERR_NETWORK — connection/auth failure
        if (code === 4 || code === 2) {
          logInfo('player', `Track ${track.id}: error ${code}, attempting token refresh and retry`);
          this.retryWithFreshToken(track, retryUrl);
          return;
        }

        logError('player', `Track ${track.id} error: ${msg}`);
        this.isPlaying = false;
        this.onPlaybackError?.(new Error(msg));
      };

      await this.audioElement.play();
      this.isPlaying = true;
      logInfo('player', `Playing track ${track.id}: ${track.title}`);
    } catch (error) {
      const err = error instanceof Error ? error : new Error(String(error));

      // "AbortError" fires when stopCurrent() interrupts a play() in progress
      // (e.g., rapidly clicking tracks). This is expected — not a real failure.
      if (err.name === 'AbortError') return;

      logError('player', `Failed to play track ${track.id}: ${err.message}`, err);
      this.isPlaying = false;
      this.onPlaybackError?.(err);
      throw err;
    }
  }

  private async retryWithFreshToken(track: Track, streamUrl: string): Promise<void> {
    // Try refreshing the access token, then retry the stream
    const refreshToken = localStorage.getItem('refreshToken');
    if (!refreshToken) {
      logError('player', 'No refresh token available for retry');
      this.onPlaybackError?.(new Error('Authentication expired. Please log in again.'));
      return;
    }

    try {
      const baseUrl = localStorage.getItem('serverUrl') || '';
      const resp = await fetch(`${baseUrl}/api/v3/auth/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken }),
      });

      if (!resp.ok) {
        logError('player', `Token refresh failed: ${resp.status}`);
        this.onPlaybackError?.(new Error('Authentication expired. Please log in again.'));
        return;
      }

      const data = await resp.json();
      localStorage.setItem('accessToken', data.accessToken);
      localStorage.setItem('refreshToken', data.refreshToken);
      logInfo('player', 'Token refreshed, retrying stream');

      // Retry with new token
      this.stopCurrent();
      this.audioElement = this.createAudioElement(streamUrl);
      this.audioElement.volume = this.volume;
      this.audioElement.playbackRate = this.playbackSpeed;

      this.audioElement.onended = () => {
        this.isPlaying = false;
        this.onPlaybackEnd?.();
      };

      this.audioElement.onerror = () => {
        const msg = this.audioElement?.error?.message || 'Playback failed after token refresh';
        logError('player', `Track ${track.id} retry failed: ${msg}`);
        this.isPlaying = false;
        this.onPlaybackError?.(new Error(msg));
      };

      await this.audioElement.play();
      this.isPlaying = true;
      logInfo('player', `Playing track ${track.id} after token refresh`);
    } catch (error) {
      logError('player', 'Token refresh error', error);
      this.onPlaybackError?.(new Error('Authentication expired. Please log in again.'));
    }
  }

  private stopCurrent(): void {
    if (this.audioElement) {
      this.audioElement.pause();
      this.audioElement.onended = null;
      this.audioElement.onerror = null;
      this.audioElement.removeAttribute('src');
      this.audioElement.load();
    }
    this.audioElement = null;
    this.isPlaying = false;
  }

  async preloadNext(track: Track, streamUrl: string): Promise<void> {
    try {
      if (this.nextAudioElement) {
        this.nextAudioElement.removeAttribute('src');
        this.nextAudioElement.load();
      }
      this.nextAudioElement = this.createAudioElement(streamUrl);
      this.nextTrack = track;
    } catch (error) {
      logError('player', `Failed to preload track ${track.id}`, error);
    }
  }

  play(): void {
    if (!this.audioElement) return;
    void this.audioElement.play();
    this.isPlaying = true;
  }

  pause(): void {
    if (!this.audioElement) return;
    this.audioElement.pause();
    this.isPlaying = false;
  }

  stop(): void {
    this.stopCurrent();
  }

  replay(): void {
    if (!this.audioElement) return;
    this.audioElement.currentTime = 0;
    void this.audioElement.play();
    this.isPlaying = true;
  }

  seek(timeSeconds: number): void {
    if (!this.audioElement) return;
    this.audioElement.currentTime = timeSeconds;
  }

  setVolume(vol: number): void {
    this.volume = Math.max(0, Math.min(1, vol));
    if (this.audioElement) {
      this.audioElement.volume = this.volume;
    }
  }

  setPlaybackSpeed(speed: number): void {
    this.playbackSpeed = Math.max(0.5, Math.min(2, speed));
    if (this.audioElement) {
      this.audioElement.playbackRate = this.playbackSpeed;
    }
  }

  getCurrentTime(): number {
    return this.audioElement?.currentTime ?? 0;
  }

  getDuration(): number {
    const d = this.audioElement?.duration ?? 0;
    return isFinite(d) ? d : 0;
  }

  getPlaybackState(): boolean {
    return this.isPlaying;
  }

  getPlaybackInfo(): {
    bufferedPercent: number;
    networkState: string;
    readyState: string;
  } | null {
    if (!this.audioElement) return null;

    const duration = this.audioElement.duration;
    let bufferedPercent = 0;
    if (duration > 0 && this.audioElement.buffered.length > 0) {
      bufferedPercent = (this.audioElement.buffered.end(this.audioElement.buffered.length - 1) / duration) * 100;
    }

    const networkStates = ['EMPTY', 'IDLE', 'LOADING', 'NO_SOURCE'];
    const readyStates = ['NOTHING', 'METADATA', 'CURRENT_DATA', 'FUTURE_DATA', 'ENOUGH_DATA'];

    return {
      bufferedPercent: Math.round(bufferedPercent),
      networkState: networkStates[this.audioElement.networkState] || 'UNKNOWN',
      readyState: readyStates[this.audioElement.readyState] || 'UNKNOWN',
    };
  }

  // --- Stubs for Web Audio features (not yet implemented) ---
  getPipelineState(): AudioPipelineState | null { return null; }
  getEqualizer(): EqualizerProcessor | null { return null; }
  getCompressor(): DynamicsCompressorNode | null { return null; }
  getAnalyserNode(): AnalyserNode | null { return null; }
  getAudioContext(): AudioContext | null { return null; }
  setCompressorParams(_params: Record<string, number>): void { /* no-op */ }
  setCompressorEnabled(_enabled: boolean): void { /* no-op */ }
  setReplayGain(_gainDb: number | null): void { /* no-op */ }
  setLimiterEnabled(_enabled: boolean): void { /* no-op */ }
  getReplayGainNode(): GainNode | null { return null; }
  getLimiterNode(): DynamicsCompressorNode | null { return null; }
  startCrossfade(): void { /* no-op */ }
  cancelCrossfade(): void { /* no-op */ }
  getIsCrossfading(): boolean { return false; }

  setPlaybackEndCallback(callback: () => void): void {
    this.onPlaybackEnd = callback;
  }

  setPlaybackErrorCallback(callback: (error: Error) => void): void {
    this.onPlaybackError = callback;
  }

  async close(): Promise<void> {
    this.stopCurrent();
    if (this.nextAudioElement) {
      this.nextAudioElement.removeAttribute('src');
      this.nextAudioElement.load();
      this.nextAudioElement = null;
    }
    this.nextTrack = null;
  }
}
