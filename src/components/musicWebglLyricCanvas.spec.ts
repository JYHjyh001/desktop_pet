import { describe, expect, it } from 'vitest'
import { ensureCanvasBitmapSize } from './musicWebglLyricCanvas'

describe('ensureCanvasBitmapSize', () => {
  it('does not reset the canvas bitmap when a progress-only redraw keeps the same size', () => {
    let width = 1536
    let height = 224
    let widthWrites = 0
    let heightWrites = 0
    const canvas = {
      get width() {
        return width
      },
      set width(value: number) {
        width = value
        widthWrites += 1
      },
      get height() {
        return height
      },
      set height(value: number) {
        height = value
        heightWrites += 1
      },
    }

    expect(ensureCanvasBitmapSize(canvas, 1536, 224)).toBe(false)
    expect(ensureCanvasBitmapSize(canvas, 1536, 224)).toBe(false)
    expect(widthWrites).toBe(0)
    expect(heightWrites).toBe(0)
  })

  it('only resizes the dimension that changed', () => {
    const canvas = { width: 1280, height: 192 }

    expect(ensureCanvasBitmapSize(canvas, 1280, 228)).toBe(true)
    expect(canvas).toEqual({ width: 1280, height: 228 })
  })
})
