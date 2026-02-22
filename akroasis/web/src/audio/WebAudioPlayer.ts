// Web Audio API player with gapless playback
import type { Track } from '../types';
import { EqualizerProcessor } from './EqualizerProcessor';

export interface AudioPipelineState {
  inputFormat: {
    sampleRate: number;
    bitDepth: number;
    channels: number;
    codec: string;
  };
  outputDevice: {
    sampleRate: number;
    channels: number;
  };
  bufferSize: number;
  latency: number;
}

export class WebAudioPlayer {
  private audioContext: AudioContext | null = null;
  private primarySource: AudioBufferSourceNode | null = null;
  private gainNode: GainNode | null = null;
  private equalizer: EqualizerProcessor | null = null;

  private compressorNode: DynamicsCompressorNode | null = null;
  private analyserNode: AnalyserNode | null = null;

  private currentTrack: Track | null = null;
  private nextTrack: Track | null = null;
  private nextBuffer: AudioBuffer | null = null;

  private currentBuffer: AudioBuffer | null = null;
  private isPlaying = false;
  private isPaused = false;
  private isManuallyStopped = false;
  private startTime = 0;
  private pauseTime = 0;
  private playbackSpeed = 1;

  // Callbacks
  private onPlaybackEnd?: () => void;
  private onPlaybackError?: (error: Error) => void;

  constructor() {
    // Initialize Audio Context lazily to avoid autoplay restrictions
  }

  private initializeContext(): void {
    if (this.audioContext) return;

    const AudioContextClass = globalThis.AudioContext || (globalThis as typeof globalThis & { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    this.audioContext = new AudioContextClass();

    // Create gain node for volume control
    this.gainNode = this.audioContext.createGain();
    this.gainNode.connect(this.audioContext.destination);

    // Create compressor: EQ → compressor → gainNode → destination
    this.compressorNode = this.audioContext.createDynamicsCompressor();
    this.compressorNode.connect(this.gainNode);

    // Create analyser as passive tap from compressor output (post-compression, pre-volume)
    this.analyserNode = this.audioContext.createAnalyser();
    this.analyserNode.fftSize = 2048;
    this.compressorNode.connect(this.analyserNode);

    // Insert EQ chain: source → eq.inputNode → [filters] → compressor → gainNode → destination
    this.equalizer = new EqualizerProcessor(this.audioContext);
    this.equalizer.connect(this.compressorNode);
  }

  async loadTrack(track: Track, streamUrl: string): Promise<void> {
    this.initializeContext();
    if (!this.audioContext) throw new Error('AudioContext initialization failed');

    if (this.audioContext.state === 'suspended') {
      await this.audioContext.resume();
    }

    try {
      const response = await fetch(streamUrl);
      if (!response.ok) throw new Error(`Failed to fetch audio: ${response.statusText}`);

      const arrayBuffer = await response.arrayBuffer();
      const audioBuffer = await this.audioContext.decodeAudioData(arrayBuffer);

      this.currentTrack = track;
      return this.playBuffer(audioBuffer);
    } catch (error) {
      const err = error instanceof Error ? error : new Error(String(error));
      this.onPlaybackError?.(err);
      throw err;
    }
  }

  private playBuffer(buffer: AudioBuffer): void {
    if (!this.audioContext || !this.gainNode) return;

    // Stop previous source if exists
    if (this.primarySource) {
      this.isManuallyStopped = true;
      try {
        this.primarySource.stop();
        this.primarySource.disconnect();
      } catch (error) {
        console.error('Failed to stop audio source:', error);
      }
    }

    // Create new source — connect to EQ input if available, else directly to gain
    const source = this.audioContext.createBufferSource();
    source.buffer = buffer;
    this.currentBuffer = buffer;
    source.playbackRate.value = this.playbackSpeed;
    const connectTarget = this.equalizer ? this.equalizer.getInputNode() : this.gainNode;
    source.connect(connectTarget);

    // Setup ended callback for gapless transition (guard against manual stop/pause/seek)
    source.onended = () => {
      if (this.isManuallyStopped) {
        this.isManuallyStopped = false;
        return;
      }

      if (this.nextBuffer && this.nextTrack) {
        this.currentTrack = this.nextTrack;
        this.nextTrack = null;
        this.playBuffer(this.nextBuffer);
        this.nextBuffer = null;
        this.onPlaybackEnd?.();
      } else {
        this.isPlaying = false;
        this.onPlaybackEnd?.();
      }
    };

    // Start playback
    source.start(0, this.pauseTime);
    this.primarySource = source;
    this.isPlaying = true;
    this.isPaused = false;
    this.startTime = this.audioContext.currentTime - this.pauseTime;
  }

  async preloadNext(track: Track, streamUrl: string): Promise<void> {
    this.initializeContext();
    if (!this.audioContext) return;

    try {
      const response = await fetch(streamUrl);
      if (!response.ok) {
        console.warn(`Failed to preload next track: ${response.statusText}`);
        return;
      }

      const arrayBuffer = await response.arrayBuffer();
      this.nextBuffer = await this.audioContext.decodeAudioData(arrayBuffer);
      this.nextTrack = track;
    } catch (error) {
      console.warn('Preload failed:', error);
      // Non-critical error - don't stop playback
    }
  }

  play(): void {
    if (!this.audioContext) {
      console.warn('No audio context - load a track first');
      return;
    }

    if (this.audioContext.state === 'suspended') {
      void this.audioContext.resume();
    }

    if (this.isPaused && this.currentTrack && this.currentBuffer) {
      // Recreate source node — stopped AudioBufferSourceNodes cannot be restarted
      this.playBuffer(this.currentBuffer);
    }
  }

  pause(): void {
    if (!this.audioContext || !this.isPlaying) return;

    this.pauseTime = this.getCurrentTime();
    this.isPaused = true;
    this.isPlaying = false;

    if (this.primarySource) {
      this.isManuallyStopped = true;
      try {
        this.primarySource.stop();
        this.primarySource.disconnect();
      } catch (error) {
        console.error('Failed to stop audio source:', error);
      }
    }
  }

  stop(): void {
    if (this.primarySource) {
      this.isManuallyStopped = true;
      try {
        this.primarySource.stop();
        this.primarySource.disconnect();
      } catch (error) {
        console.error('Failed to stop audio source:', error);
      }
      this.primarySource = null;
    }

    this.isPlaying = false;
    this.isPaused = false;
    this.pauseTime = 0;
    this.currentTrack = null;
    this.currentBuffer = null;
  }

  seek(timeSeconds: number): void {
    if (!this.currentTrack || !this.audioContext) return;

    this.pauseTime = timeSeconds;

    if (this.isPlaying && this.primarySource?.buffer) {
      // Restart from new position
      this.playBuffer(this.primarySource.buffer);
    }
  }

  setVolume(volume: number): void {
    if (!this.gainNode) return;

    // Clamp volume to 0-1 range
    const clampedVolume = Math.max(0, Math.min(1, volume));
    this.gainNode.gain.value = clampedVolume;
  }

  setPlaybackSpeed(speed: number): void {
    // Clamp playback speed to 0.5-2 range
    const clampedSpeed = Math.max(0.5, Math.min(2, speed));
    this.playbackSpeed = clampedSpeed;

    // Update current source if playing
    if (this.primarySource && this.isPlaying) {
      this.primarySource.playbackRate.value = clampedSpeed;
    }
  }

  getCurrentTime(): number {
    if (!this.audioContext || !this.isPlaying) return this.pauseTime;
    return this.audioContext.currentTime - this.startTime;
  }

  getDuration(): number {
    return this.primarySource?.buffer?.duration ?? 0;
  }

  getPlaybackState(): boolean {
    return this.isPlaying;
  }

  getPipelineState(): AudioPipelineState | null {
    if (!this.audioContext || !this.primarySource?.buffer) return null;

    return {
      inputFormat: {
        sampleRate: this.primarySource.buffer.sampleRate,
        bitDepth: 16, // Web Audio API doesn't expose bit depth
        channels: this.primarySource.buffer.numberOfChannels,
        codec: 'decoded-pcm' // Browser handles decoding
      },
      outputDevice: {
        sampleRate: this.audioContext.sampleRate,
        channels: this.audioContext.destination.maxChannelCount
      },
      bufferSize: this.primarySource.buffer.length,
      latency: this.audioContext.baseLatency ?? 0
    };
  }

  getEqualizer(): EqualizerProcessor | null {
    return this.equalizer;
  }

  getCompressor(): DynamicsCompressorNode | null {
    return this.compressorNode;
  }

  getAnalyserNode(): AnalyserNode | null {
    return this.analyserNode;
  }

  setCompressorParams(params: {
    threshold?: number;
    knee?: number;
    ratio?: number;
    attack?: number;
    release?: number;
  }): void {
    if (!this.compressorNode) return;
    if (params.threshold !== undefined) this.compressorNode.threshold.value = params.threshold;
    if (params.knee !== undefined) this.compressorNode.knee.value = params.knee;
    if (params.ratio !== undefined) this.compressorNode.ratio.value = params.ratio;
    if (params.attack !== undefined) this.compressorNode.attack.value = params.attack;
    if (params.release !== undefined) this.compressorNode.release.value = params.release;
  }

  setCompressorEnabled(enabled: boolean): void {
    if (!this.compressorNode) return;
    if (!enabled) {
      // Bypass by setting ratio to 1 (passthrough) — avoids graph rewiring
      this.compressorNode.ratio.value = 1;
      this.compressorNode.threshold.value = 0;
    }
    // When enabling, caller must also call setCompressorParams with current values
  }

  setPlaybackEndCallback(callback: () => void): void {
    this.onPlaybackEnd = callback;
  }

  setPlaybackErrorCallback(callback: (error: Error) => void): void {
    this.onPlaybackError = callback;
  }

  async close(): Promise<void> {
    this.stop();

    if (this.analyserNode) {
      this.analyserNode.disconnect();
      this.analyserNode = null;
    }

    if (this.compressorNode) {
      this.compressorNode.disconnect();
      this.compressorNode = null;
    }

    if (this.equalizer) {
      this.equalizer.disconnect();
      this.equalizer = null;
    }

    if (this.audioContext) {
      await this.audioContext.close();
      this.audioContext = null;
    }

    this.gainNode = null;
    this.currentBuffer = null;
    this.nextBuffer = null;
    this.nextTrack = null;
  }
}
