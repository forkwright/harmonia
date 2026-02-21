// LRC format parser — [mm:ss.xx] and [mm:ss] timestamps
export interface LrcLine {
  timeMs: number
  text: string
}

const TIMESTAMP_RE = /\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g

function parseTimestamp(mm: string, ss: string, frac?: string): number {
  const minutes = parseInt(mm, 10)
  const seconds = parseInt(ss, 10)
  let ms = 0
  if (frac) {
    // Normalise fractional part: 2 digits → centiseconds, 3 digits → milliseconds
    ms = frac.length === 3 ? parseInt(frac, 10) : parseInt(frac, 10) * 10
  }
  return (minutes * 60 + seconds) * 1000 + ms
}

export function parseLrc(lrc: string): LrcLine[] {
  const lines: LrcLine[] = []

  for (const rawLine of lrc.split('\n')) {
    const text = rawLine.replace(TIMESTAMP_RE, '').trim()

    // Reset regex state for re-use on each line
    TIMESTAMP_RE.lastIndex = 0
    let match: RegExpExecArray | null

    while ((match = TIMESTAMP_RE.exec(rawLine)) !== null) {
      const timeMs = parseTimestamp(match[1], match[2], match[3])
      lines.push({ timeMs, text })
    }
  }

  return lines
    .filter((l) => l.text.length > 0)
    .sort((a, b) => a.timeMs - b.timeMs)
}

export function findActiveLine(lines: LrcLine[], positionMs: number): number {
  if (lines.length === 0) return -1

  let active = 0
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].timeMs <= positionMs) {
      active = i
    } else {
      break
    }
  }
  return active
}
