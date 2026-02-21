import { describe, it, expect } from 'vitest'
import { parseLrc, findActiveLine } from './lrcParser'

describe('parseLrc', () => {
  it('parses basic [mm:ss.xx] timestamps', () => {
    const lrc = '[00:10.50]Hello world\n[00:20.00]Second line'
    const result = parseLrc(lrc)
    expect(result).toHaveLength(2)
    expect(result[0]).toEqual({ timeMs: 10500, text: 'Hello world' })
    expect(result[1]).toEqual({ timeMs: 20000, text: 'Second line' })
  })

  it('parses [mm:ss] without fractional seconds', () => {
    const lrc = '[01:30]No fractions here'
    const result = parseLrc(lrc)
    expect(result).toHaveLength(1)
    expect(result[0]).toEqual({ timeMs: 90000, text: 'No fractions here' })
  })

  it('parses three-digit centiseconds as milliseconds', () => {
    const lrc = '[00:05.123]Three digits'
    const result = parseLrc(lrc)
    expect(result[0].timeMs).toBe(5123)
  })

  it('parses two-digit centiseconds correctly', () => {
    const lrc = '[00:05.75]Centiseconds'
    const result = parseLrc(lrc)
    // 75 centiseconds = 750ms
    expect(result[0].timeMs).toBe(5750)
  })

  it('handles multi-tag lines (same text, multiple timestamps)', () => {
    const lrc = '[00:10.00][00:30.00]Chorus line'
    const result = parseLrc(lrc)
    expect(result).toHaveLength(2)
    expect(result[0]).toEqual({ timeMs: 10000, text: 'Chorus line' })
    expect(result[1]).toEqual({ timeMs: 30000, text: 'Chorus line' })
  })

  it('sorts lines by ascending timestamp', () => {
    const lrc = '[00:30.00]Third\n[00:10.00]First\n[00:20.00]Second'
    const result = parseLrc(lrc)
    expect(result[0].text).toBe('First')
    expect(result[1].text).toBe('Second')
    expect(result[2].text).toBe('Third')
  })

  it('filters out empty lines', () => {
    const lrc = '[00:10.00]  \n[00:20.00]Real line\n[00:30.00]'
    const result = parseLrc(lrc)
    expect(result).toHaveLength(1)
    expect(result[0].text).toBe('Real line')
  })

  it('strips metadata tags (ar, ti, al)', () => {
    // Metadata tags like [ar:Artist] have no timestamp digits in mm:ss format
    const lrc = '[00:01.00]First lyric'
    const result = parseLrc(lrc)
    expect(result).toHaveLength(1)
    expect(result[0].text).toBe('First lyric')
  })

  it('returns empty array for empty input', () => {
    expect(parseLrc('')).toHaveLength(0)
  })

  it('handles large minute values', () => {
    const lrc = '[75:30.00]Long track line'
    const result = parseLrc(lrc)
    expect(result[0].timeMs).toBe((75 * 60 + 30) * 1000)
  })

  it('handles real-world LRC block', () => {
    const lrc = [
      '[00:00.00]',
      '[00:17.32]Never gonna give you up',
      '[00:20.11]Never gonna let you down',
      '[00:22.87]Never gonna run around and desert you',
    ].join('\n')
    const result = parseLrc(lrc)
    expect(result).toHaveLength(3)
    expect(result[0].text).toBe('Never gonna give you up')
    expect(result[2].text).toBe('Never gonna run around and desert you')
  })
})

describe('findActiveLine', () => {
  const lines = [
    { timeMs: 0, text: 'Intro' },
    { timeMs: 5000, text: 'Verse 1' },
    { timeMs: 10000, text: 'Chorus' },
    { timeMs: 20000, text: 'Verse 2' },
  ]

  it('returns -1 for empty lines array', () => {
    expect(findActiveLine([], 5000)).toBe(-1)
  })

  it('returns 0 when position is at start', () => {
    expect(findActiveLine(lines, 0)).toBe(0)
  })

  it('returns first line when position is between start and second line', () => {
    expect(findActiveLine(lines, 3000)).toBe(0)
  })

  it('returns correct line at exact timestamp', () => {
    expect(findActiveLine(lines, 10000)).toBe(2)
  })

  it('returns correct line between timestamps', () => {
    expect(findActiveLine(lines, 12000)).toBe(2)
    expect(findActiveLine(lines, 19999)).toBe(2)
  })

  it('returns last line past the end', () => {
    expect(findActiveLine(lines, 99000)).toBe(3)
  })

  it('handles single-line array', () => {
    expect(findActiveLine([{ timeMs: 1000, text: 'Only' }], 500)).toBe(0)
    expect(findActiveLine([{ timeMs: 1000, text: 'Only' }], 1000)).toBe(0)
    expect(findActiveLine([{ timeMs: 1000, text: 'Only' }], 5000)).toBe(0)
  })
})
