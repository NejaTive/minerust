import { block_icon } from '@engine'

/** Сторона тайла в пикселях; иконка — тот же тайл, что на боковой грани блока в мире. */
export const BLOCK_ICON_SIZE = 16

/** Рисует иконку блока в canvas; неизвестное имя — no-op. */
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

/** Все `canvas[data-block]` внутри корня — по `data-block`, порядок ref'ов Vue не гарантирует. */
export function paintBlockIconsIn(root: HTMLElement | null | undefined) {
  root?.querySelectorAll<HTMLCanvasElement>('canvas[data-block]').forEach((canvas) => {
    paintBlockIcon(canvas, canvas.dataset.block ?? '')
  })
}
