// Tests for shared localStorage helpers
import { describe, it, expect, beforeEach } from 'vitest'
import { loadJson, saveJson } from './storage'

describe('storage utils', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  describe('loadJson', () => {
    it('returns parsed JSON when key exists', () => {
      localStorage.setItem('test_key', JSON.stringify({ foo: 'bar' }))
      expect(loadJson('test_key', {})).toEqual({ foo: 'bar' })
    })

    it('returns fallback when key does not exist', () => {
      expect(loadJson('missing', 42)).toBe(42)
    })

    it('returns fallback when stored value is invalid JSON', () => {
      localStorage.setItem('bad_json', '{not valid')
      expect(loadJson('bad_json', 'default')).toBe('default')
    })

    it('handles array values', () => {
      localStorage.setItem('arr', JSON.stringify([1, 2, 3]))
      expect(loadJson<number[]>('arr', [])).toEqual([1, 2, 3])
    })

    it('handles null stored value', () => {
      localStorage.setItem('null_val', 'null')
      expect(loadJson('null_val', 'fallback')).toBeNull()
    })
  })

  describe('saveJson', () => {
    it('stores value as JSON string', () => {
      saveJson('test_key', { foo: 'bar' })
      expect(localStorage.getItem('test_key')).toBe('{"foo":"bar"}')
    })

    it('stores arrays', () => {
      saveJson('arr', [1, 2, 3])
      expect(localStorage.getItem('arr')).toBe('[1,2,3]')
    })

    it('stores primitives', () => {
      saveJson('num', 42)
      expect(localStorage.getItem('num')).toBe('42')
    })

    it('overwrites existing values', () => {
      saveJson('key', 'first')
      saveJson('key', 'second')
      expect(loadJson('key', '')).toBe('second')
    })
  })
})
