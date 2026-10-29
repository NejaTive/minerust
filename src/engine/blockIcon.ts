import { block_icon } from '@engine'

export const BLOCK_ICON_SIZE = 16

export function paintBlockIcon(canvas: HTMLCanvasElement, name: string) {
  const pixels = block_icon(name)
  if (!pixels) return
  canvas
    .getContext('2d')
    ?.putImageData(
      new ImageData(new Uint8ClampedArray(pixels), BLOCK_ICON_SIZE, BLOCK_ICON_SIZE),
      0,
      0,
    )
}

export function paintBlockIconsIn(root: HTMLElement | null | undefined) {
  root?.querySelectorAll<HTMLCanvasElement>('canvas[data-block]').forEach((canvas) => {
    paintBlockIcon(canvas, canvas.dataset.block ?? '')
  })
}
