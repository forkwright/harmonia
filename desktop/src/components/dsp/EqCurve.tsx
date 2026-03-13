import { useMemo, useCallback, useRef } from "react";
import type { EqBand } from "../../types/dsp";
import {
  computeCombinedResponse,
  generateFrequencyPoints,
} from "./biquad";

const SAMPLE_RATE = 44100;
const MIN_HZ = 20;
const MAX_HZ = 20000;
const MIN_DB = -15;
const MAX_DB = 15;
const CURVE_POINTS = 256;

const PADDING = { top: 20, right: 20, bottom: 30, left: 45 };
const GRID_FREQS = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const GRID_LABELS = [
  "31",
  "63",
  "125",
  "250",
  "500",
  "1k",
  "2k",
  "4k",
  "8k",
  "16k",
];
const DB_GRID = [-12, -9, -6, -3, 0, 3, 6, 9, 12];

function freqToX(freq: number, width: number): number {
  const logMin = Math.log10(MIN_HZ);
  const logMax = Math.log10(MAX_HZ);
  const t = (Math.log10(freq) - logMin) / (logMax - logMin);
  return PADDING.left + t * width;
}

function dbToY(db: number, height: number): number {
  const t = (db - MAX_DB) / (MIN_DB - MAX_DB);
  return PADDING.top + t * height;
}

function yToDb(y: number, height: number): number {
  const t = (y - PADDING.top) / height;
  return MAX_DB + t * (MIN_DB - MAX_DB);
}

function xToFreq(x: number, width: number): number {
  const logMin = Math.log10(MIN_HZ);
  const logMax = Math.log10(MAX_HZ);
  const t = (x - PADDING.left) / width;
  return Math.pow(10, logMin + t * (logMax - logMin));
}

interface EqCurveProps {
  bands: EqBand[];
  preampDb: number;
  selectedIndex: number | null;
  onBandChange: (index: number, updates: Partial<EqBand>) => void;
  onBandSelect: (index: number | null) => void;
}

export default function EqCurve({
  bands,
  preampDb,
  selectedIndex,
  onBandChange,
  onBandSelect,
}: EqCurveProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const dragRef = useRef<{
    index: number;
    startX: number;
    startY: number;
    startFreq: number;
    startGain: number;
  } | null>(null);

  const frequencies = useMemo(
    () => generateFrequencyPoints(CURVE_POINTS, MIN_HZ, MAX_HZ),
    [],
  );

  const response = useMemo(
    () => computeCombinedResponse(bands, frequencies, SAMPLE_RATE, preampDb),
    [bands, frequencies, preampDb],
  );

  const svgWidth = 800;
  const svgHeight = 320;
  const plotWidth = svgWidth - PADDING.left - PADDING.right;
  const plotHeight = svgHeight - PADDING.top - PADDING.bottom;

  const curvePath = useMemo(() => {
    const points = frequencies.map((freq, i) => {
      const x = freqToX(freq, plotWidth);
      const clampedDb = Math.max(MIN_DB, Math.min(MAX_DB, response[i]));
      const y = dbToY(clampedDb, plotHeight);
      return `${x},${y}`;
    });
    return `M${points.join("L")}`;
  }, [frequencies, response, plotWidth, plotHeight]);

  const fillPath = useMemo(() => {
    const zeroY = dbToY(0, plotHeight);
    const firstX = freqToX(frequencies[0], plotWidth);
    const lastX = freqToX(frequencies[frequencies.length - 1], plotWidth);
    return `${curvePath}L${lastX},${zeroY}L${firstX},${zeroY}Z`;
  }, [curvePath, frequencies, plotWidth, plotHeight]);

  const hasClipping = response.some((db) => db > 0);

  const clippingPath = useMemo(() => {
    if (!hasClipping) return null;
    const zeroY = dbToY(0, plotHeight);
    const segments: string[] = [];
    let inClip = false;
    let segStart = "";

    for (let i = 0; i < frequencies.length; i++) {
      const x = freqToX(frequencies[i], plotWidth);
      const db = Math.max(MIN_DB, Math.min(MAX_DB, response[i]));
      const y = dbToY(db, plotHeight);

      if (response[i] > 0) {
        if (!inClip) {
          segStart = `M${x},${zeroY}L${x},${y}`;
          inClip = true;
        } else {
          segStart += `L${x},${y}`;
        }
      } else if (inClip) {
        segStart += `L${x},${zeroY}Z`;
        segments.push(segStart);
        inClip = false;
      }
    }
    if (inClip) {
      const lastX = freqToX(frequencies[frequencies.length - 1], plotWidth);
      segStart += `L${lastX},${zeroY}Z`;
      segments.push(segStart);
    }
    return segments.join("");
  }, [hasClipping, frequencies, response, plotWidth, plotHeight]);

  const getSvgPoint = useCallback(
    (e: React.MouseEvent | MouseEvent) => {
      const svg = svgRef.current;
      if (!svg) return { x: 0, y: 0 };
      const rect = svg.getBoundingClientRect();
      const scaleX = svgWidth / rect.width;
      const scaleY = svgHeight / rect.height;
      return {
        x: (e.clientX - rect.left) * scaleX,
        y: (e.clientY - rect.top) * scaleY,
      };
    },
    [svgWidth, svgHeight],
  );

  const handleMarkerDown = useCallback(
    (e: React.MouseEvent, index: number) => {
      e.preventDefault();
      e.stopPropagation();
      onBandSelect(index);
      const pt = getSvgPoint(e);
      const band = bands[index];
      dragRef.current = {
        index,
        startX: pt.x,
        startY: pt.y,
        startFreq: band.frequency,
        startGain: band.gain_db,
      };

      const handleMove = (ev: MouseEvent) => {
        const drag = dragRef.current;
        if (!drag) return;
        const p = getSvgPoint(ev);
        const newFreq = xToFreq(
          freqToX(drag.startFreq, plotWidth) + (p.x - drag.startX),
          plotWidth,
        );
        const newGain =
          drag.startGain +
          yToDb(drag.startY, plotHeight) -
          yToDb(p.y, plotHeight);
        onBandChange(drag.index, {
          frequency: Math.max(MIN_HZ, Math.min(MAX_HZ, Math.round(newFreq))),
          gain_db: Math.max(
            MIN_DB,
            Math.min(MAX_DB, Math.round(newGain * 10) / 10),
          ),
        });
      };

      const handleUp = () => {
        dragRef.current = null;
        window.removeEventListener("mousemove", handleMove);
        window.removeEventListener("mouseup", handleUp);
      };

      window.addEventListener("mousemove", handleMove);
      window.addEventListener("mouseup", handleUp);
    },
    [bands, onBandChange, onBandSelect, getSvgPoint, plotWidth, plotHeight],
  );

  const handleWheel = useCallback(
    (e: React.WheelEvent, index: number) => {
      e.preventDefault();
      const band = bands[index];
      const delta = e.deltaY > 0 ? -0.1 : 0.1;
      const newQ = Math.max(0.1, Math.min(10, band.q + delta));
      onBandChange(index, { q: Math.round(newQ * 10) / 10 });
    },
    [bands, onBandChange],
  );

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${svgWidth} ${svgHeight}`}
      className="w-full bg-gray-900 rounded-lg"
    >
      {/* Grid lines - frequency */}
      {GRID_FREQS.map((freq, i) => {
        const x = freqToX(freq, plotWidth);
        return (
          <g key={freq}>
            <line
              x1={x}
              y1={PADDING.top}
              x2={x}
              y2={PADDING.top + plotHeight}
              stroke="#374151"
              strokeWidth="0.5"
            />
            <text
              x={x}
              y={svgHeight - 6}
              textAnchor="middle"
              fill="#6b7280"
              fontSize="10"
            >
              {GRID_LABELS[i]}
            </text>
          </g>
        );
      })}

      {/* Grid lines - dB */}
      {DB_GRID.map((db) => {
        const y = dbToY(db, plotHeight);
        const isZero = db === 0;
        return (
          <g key={db}>
            <line
              x1={PADDING.left}
              y1={y}
              x2={PADDING.left + plotWidth}
              y2={y}
              stroke={isZero ? "#4b5563" : "#374151"}
              strokeWidth={isZero ? 1 : 0.5}
            />
            <text
              x={PADDING.left - 6}
              y={y + 3}
              textAnchor="end"
              fill="#6b7280"
              fontSize="10"
            >
              {db > 0 ? `+${db}` : db}
            </text>
          </g>
        );
      })}

      {/* Preamp line */}
      {preampDb !== 0 && (
        <line
          x1={PADDING.left}
          y1={dbToY(preampDb, plotHeight)}
          x2={PADDING.left + plotWidth}
          y2={dbToY(preampDb, plotHeight)}
          stroke="#6366f1"
          strokeWidth="1"
          strokeDasharray="4 3"
          opacity={0.6}
        />
      )}

      {/* Clipping area */}
      {clippingPath && (
        <path d={clippingPath} fill="rgba(239, 68, 68, 0.15)" />
      )}

      {/* Response curve fill */}
      <path d={fillPath} fill="rgba(99, 102, 241, 0.1)" />

      {/* Response curve */}
      <path
        d={curvePath}
        fill="none"
        stroke="#6366f1"
        strokeWidth="2"
      />

      {/* Band markers */}
      {bands.map((band, i) => {
        if (!band.enabled) return null;
        const x = freqToX(band.frequency, plotWidth);
        const bandResponse = computeCombinedResponse(
          [band],
          [band.frequency],
          SAMPLE_RATE,
          0,
        )[0];
        const y = dbToY(
          Math.max(
            MIN_DB,
            Math.min(MAX_DB, bandResponse + preampDb),
          ),
          plotHeight,
        );
        const isSelected = selectedIndex === i;
        return (
          <circle
            key={i}
            cx={x}
            cy={y}
            r={isSelected ? 7 : 5}
            fill={isSelected ? "#818cf8" : "#6366f1"}
            stroke={isSelected ? "#c7d2fe" : "#a5b4fc"}
            strokeWidth={isSelected ? 2 : 1}
            className="cursor-grab active:cursor-grabbing"
            onMouseDown={(e) => handleMarkerDown(e, i)}
            onWheel={(e) => handleWheel(e, i)}
          />
        );
      })}

      {/* Plot border */}
      <rect
        x={PADDING.left}
        y={PADDING.top}
        width={plotWidth}
        height={plotHeight}
        fill="none"
        stroke="#374151"
        strokeWidth="1"
      />
    </svg>
  );
}
