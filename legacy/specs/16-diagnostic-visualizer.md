# Spec 16: diagnostic visualizer

**Status:** Draft
**Priority:** Low

## Goal

Add a diagnostic spectrum analyzer to the EQ panel that verifies audio signal integrity: confirm EQ curves are applying, detect clipping, and identify lossy encoding disguised as lossless. This is a diagnostic tool for audiophiles, not a decorative visualizer. Uses the existing AnalyserNode already in the WebAudioPlayer pipeline.

## Greek name

**Dokimastes** (do-ki-mas-TAYS): the assayer. From dokimazo (to test, to prove, to assay metals). Not visualization for aesthetics but verification. The dokimastes reveals whether the signal is what it claims to be.

## Phases

### Phase 1: spectrum analyzer core
- [ ] Create `audio/spectrumAnalyzer.ts` (SpectrumAnalyzer class wrapping existing AnalyserNode)
- [ ] Implement RMS and peak level computation (dBFS from time-domain data)
- [ ] Implement clipping detection (3+ consecutive samples at +/-0.99)
- [ ] Implement spectral rolloff calculation (85% energy frequency)
- [ ] Implement lossy encoding detection (sharp cutoff at 16/18.5/20 kHz)
- [ ] Implement peak hold with decay
- [ ] Tests with synthetic sine waves and clipping signals

### Phase 2: visualizer UI
- [ ] Create `visualizerStore.ts` (enabled, mode, fftSize, smoothing, indicators)
- [ ] Create `SpectrumVisualizer.tsx` (canvas with requestAnimationFrame)
- [ ] Implement spectrum mode (vertical bars, log frequency scale, dBFS vertical)
- [ ] Implement spectrogram mode (scrolling time-frequency plot)
- [ ] Implement waveform mode (time-domain oscilloscope)
- [ ] Add collapsible section below EQ sliders in EqualizerPanel
- [ ] Diagnostic indicators: RMS level, peak level, CLIP badge (red), LOSSY badge (yellow)
- [ ] Tests for store, visual verification manual

## Dependencies

- Existing AnalyserNode in WebAudioPlayer (passive tap, already connected)
- No backend dependencies
- No dependencies on other specs; can be built independently at any time

## Notes

- The AnalyserNode is already created in WebAudioPlayer and taps the signal between compressor and gain node. No new audio nodes needed.
- Canvas rendering (not React DOM) for performance at 60fps with large FFT sizes.
- Lossy detection: MP3 128kbps cuts at ~16kHz, AAC 256kbps at ~18.5kHz, MP3 320kbps at ~20kHz. A sharp spectral shelf at these frequencies indicates lossy encoding regardless of container format.
- This feature is independent and parallelizable; good candidate for working on alongside other batches.
- Layout: collapsible below EQ band sliders, above Dynamics section. Default collapsed.
