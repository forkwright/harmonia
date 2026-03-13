import type { EqBand } from "../../types/dsp";

interface Coefficients {
  b0: number;
  b1: number;
  b2: number;
  a1: number;
  a2: number;
}

const PASSTHROUGH: Coefficients = { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 };

export function computeCoefficients(
  band: EqBand,
  sampleRate: number,
): Coefficients {
  if (band.q <= 0 || !isFinite(band.frequency) || !isFinite(band.gain_db)) {
    return PASSTHROUGH;
  }

  const w0 = (2 * Math.PI * band.frequency) / sampleRate;
  const cosW0 = Math.cos(w0);
  const sinW0 = Math.sin(w0);
  const alpha = sinW0 / (2 * band.q);

  let b0: number,
    b1: number,
    b2: number,
    a0: number,
    a1: number,
    a2: number;

  switch (band.filter_type) {
    case "Peaking": {
      const a = Math.pow(10, band.gain_db / 40);
      b0 = 1 + alpha * a;
      b1 = -2 * cosW0;
      b2 = 1 - alpha * a;
      a0 = 1 + alpha / a;
      a1 = -2 * cosW0;
      a2 = 1 - alpha / a;
      break;
    }
    case "LowShelf": {
      const a = Math.pow(10, band.gain_db / 40);
      const tsa = 2 * Math.sqrt(a) * alpha;
      b0 = a * (a + 1 - (a - 1) * cosW0 + tsa);
      b1 = 2 * a * (a - 1 - (a + 1) * cosW0);
      b2 = a * (a + 1 - (a - 1) * cosW0 - tsa);
      a0 = a + 1 + (a - 1) * cosW0 + tsa;
      a1 = -2 * (a - 1 + (a + 1) * cosW0);
      a2 = a + 1 + (a - 1) * cosW0 - tsa;
      break;
    }
    case "HighShelf": {
      const a = Math.pow(10, band.gain_db / 40);
      const tsa = 2 * Math.sqrt(a) * alpha;
      b0 = a * (a + 1 + (a - 1) * cosW0 + tsa);
      b1 = -2 * a * (a - 1 + (a + 1) * cosW0);
      b2 = a * (a + 1 + (a - 1) * cosW0 - tsa);
      a0 = a + 1 - (a - 1) * cosW0 + tsa;
      a1 = 2 * (a - 1 - (a + 1) * cosW0);
      a2 = a + 1 - (a - 1) * cosW0 - tsa;
      break;
    }
    case "LowPass":
      b0 = (1 - cosW0) / 2;
      b1 = 1 - cosW0;
      b2 = (1 - cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    case "HighPass":
      b0 = (1 + cosW0) / 2;
      b1 = -(1 + cosW0);
      b2 = (1 + cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    case "Notch":
      b0 = 1;
      b1 = -2 * cosW0;
      b2 = 1;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    case "AllPass":
      b0 = 1 - alpha;
      b1 = -2 * cosW0;
      b2 = 1 + alpha;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
  }

  if (Math.abs(a0) < Number.EPSILON) {
    return PASSTHROUGH;
  }

  return {
    b0: b0 / a0,
    b1: b1 / a0,
    b2: b2 / a0,
    a1: a1 / a0,
    a2: a2 / a0,
  };
}

export function computeMagnitudeDb(
  coeffs: Coefficients,
  frequency: number,
  sampleRate: number,
): number {
  const w = (2 * Math.PI * frequency) / sampleRate;
  const cosW = Math.cos(w);
  const cos2W = Math.cos(2 * w);
  const sinW = Math.sin(w);
  const sin2W = Math.sin(2 * w);

  const numReal = coeffs.b0 + coeffs.b1 * cosW + coeffs.b2 * cos2W;
  const numImag = -(coeffs.b1 * sinW + coeffs.b2 * sin2W);
  const denReal = 1 + coeffs.a1 * cosW + coeffs.a2 * cos2W;
  const denImag = -(coeffs.a1 * sinW + coeffs.a2 * sin2W);

  const numMagSq = numReal * numReal + numImag * numImag;
  const denMagSq = denReal * denReal + denImag * denImag;

  if (denMagSq < Number.EPSILON) return 0;

  return 10 * Math.log10(numMagSq / denMagSq);
}

export function computeCombinedResponse(
  bands: EqBand[],
  frequencies: number[],
  sampleRate: number,
  preampDb: number,
): number[] {
  const activeBands = bands.filter((b) => b.enabled);
  const coeffsList = activeBands.map((b) =>
    computeCoefficients(b, sampleRate),
  );

  return frequencies.map((freq) => {
    let totalDb = preampDb;
    for (const coeffs of coeffsList) {
      totalDb += computeMagnitudeDb(coeffs, freq, sampleRate);
    }
    return totalDb;
  });
}

export function generateFrequencyPoints(
  count: number,
  minHz = 20,
  maxHz = 20000,
): number[] {
  const logMin = Math.log10(minHz);
  const logMax = Math.log10(maxHz);
  return Array.from({ length: count }, (_, i) => {
    const t = i / (count - 1);
    return Math.pow(10, logMin + t * (logMax - logMin));
  });
}
