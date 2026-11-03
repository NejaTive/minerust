import { block_icon, block_icon_size, item_icon, item_icon_size } from '@engine'

export function paintBlockIcon(canvas: HTMLCanvasElement, name: string) {
  const block = block_icon(name)
  const pixels = block ?? item_icon(name)
  if (!pixels) return
  const size = block ? block_icon_size() : item_icon_size()
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
