// 10-band parametric equalizer using Web Audio API BiquadFilterNodes

const ISO_BAND_FREQUENCIES = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const;

const BAND_COUNT = 10;
const Q_FACTOR = 1.414;
const GAIN_MIN = -12;
const GAIN_MAX = 12;

export class EqualizerProcessor {
  private readonly filters: BiquadFilterNode[];
  // Passthrough gain node used as the stable input node sources connect to
  private readonly inputNode: GainNode;
  private enabled: boolean = true;

  constructor(context: AudioContext) {
    this.inputNode = context.createGain();

    this.filters = ISO_BAND_FREQUENCIES.map((frequency) => {
      const filter = context.createBiquadFilter();
      filter.type = 'peaking';
      filter.frequency.value = frequency;
      filter.Q.value = Q_FACTOR;
      filter.gain.value = 0;
      return filter;
    });

    // Wire: inputNode → filter[0] → ... → filter[9]
    this.inputNode.connect(this.filters[0]);
    for (let i = 0; i < this.filters.length - 1; i++) {
      this.filters[i].connect(this.filters[i + 1]);
    }
  }

  // Connect the EQ chain between an upstream and downstream AudioNode.
  // After calling this, sources should connect to getInputNode() instead of output directly.
  connect(output: AudioNode): void {
    this.filters[this.filters.length - 1].connect(output);
  }

  disconnect(): void {
    try {
      this.filters[this.filters.length - 1].disconnect();
    } catch {
      // Ignore errors if already disconnected
    }
  }

  // The node that audio sources should connect to
  getInputNode(): GainNode {
    return this.inputNode;
  }

  setGain(bandIndex: number, dB: number): void {
    if (bandIndex < 0 || bandIndex >= BAND_COUNT) return;
    const clamped = Math.max(GAIN_MIN, Math.min(GAIN_MAX, dB));
    this.filters[bandIndex].gain.value = this.enabled ? clamped : 0;
  }

  setAllGains(gains: number[]): void {
    for (let i = 0; i < BAND_COUNT; i++) {
      const dB = gains[i] ?? 0;
      const clamped = Math.max(GAIN_MIN, Math.min(GAIN_MAX, dB));
      this.filters[i].gain.value = this.enabled ? clamped : 0;
    }
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (!enabled) {
      for (const filter of this.filters) {
        filter.gain.value = 0;
      }
    }
    // Re-apply stored gains when re-enabling — caller must call setAllGains again
  }

  getFilters(): readonly BiquadFilterNode[] {
    return this.filters;
  }

  getFrequencies(): readonly number[] {
    return ISO_BAND_FREQUENCIES;
  }

  static get bandCount(): number {
    return BAND_COUNT;
  }

  static get gainMin(): number {
    return GAIN_MIN;
  }

  static get gainMax(): number {
    return GAIN_MAX;
  }
}
