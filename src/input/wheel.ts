/** Порог прокрутки, пикселей, на один слот. Колесо мыши шлёт за щелчок 100–120, тачпад — десятки
 * мелких событий за жест: накопление не даёт одному жесту пролистать хотбар. */
export const WHEEL_STEP = 100
export const WHEEL_LINE_PX = 40
export const WHEEL_PAGE_PX = 800

/** Копит прокрутку и отдаёт шаги слота: −1, 0 или 1 на событие. */
export class WheelAccumulator {
  private accum = 0

  /** `deltaMode` — как у `WheelEvent`: 0 пиксели, 1 строки, 2 страницы. */
  push(deltaY: number, deltaMode: number): -1 | 0 | 1 {
    const unit = deltaMode === 1 ? WHEEL_LINE_PX : deltaMode === 2 ? WHEEL_PAGE_PX : 1
    const delta = deltaY * unit
    // Смена направления сбрасывает остаток, иначе первый шаг назад съедался бы им.
    if (Math.sign(delta) !== Math.sign(this.accum)) this.accum = 0
    this.accum += delta
    if (Math.abs(this.accum) < WHEEL_STEP) return 0
    // Вниз — следующий слот; за одно событие — не больше одного слота.
    const step = Math.sign(this.accum) as -1 | 1
    this.accum = 0
    return step
  }
}

/** Слот через `step` по кругу. */
export function stepSlot(slot: number, step: number, count: number): number {
  return (slot + step + count) % count
}
