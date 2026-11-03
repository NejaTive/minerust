import { block_icon, block_icon_size } from '@engine'

export function paintBlockIcon(canvas: HTMLCanvasElement, name: string) {
  const pixels = block_icon(name)
  if (!pixels) return
  const size = block_icon_size()
  if (canvas.width !== size) canvas.width = canvas.height = size
  canvas
    .getContext('2d')
    ?.putImageData(new ImageData(new Uint8ClampedArray(pixels), size, size), 0, 0)
}

export function paintBlockIconsIn(root: HTMLElement | null | undefined) {
  root?.querySelectorAll<HTMLCanvasElement>('canvas[data-block]').forEach((canvas) => {
    paintBlockIcon(canvas, canvas.dataset.block ?? '')
  })
}
