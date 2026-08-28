export function ensureCanvasBitmapSize(
  canvas: Pick<HTMLCanvasElement, 'width' | 'height'>,
  width: number,
  height: number,
) {
  const nextWidth = Math.max(1, Math.round(width))
  const nextHeight = Math.max(1, Math.round(height))
  let resized = false

  if (canvas.width !== nextWidth) {
    canvas.width = nextWidth
    resized = true
  }
  if (canvas.height !== nextHeight) {
    canvas.height = nextHeight
    resized = true
  }

  return resized
}
