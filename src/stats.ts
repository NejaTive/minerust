/** Порядок полей — как в `Engine.stats()`; время — в микросекундах, если не сказано иначе. */
export interface WorldStats {
  columns: number
  meshedColumns: number
  meshedSections: number
  queued: number
  generateUs: number
  meshUs: number
  /** Секций, прошедших отсечение по пирамиде видимости, в последнем кадре. */
  drawnSections: number
  /** Свет колонки при подгрузке. */
  lightUs: number
  /** Правка блока со светом и перестройкой. */
  editUs: number
  lightKib: number
  /** CPU за кадр: подгрузка и меш в бюджете. Нули без `set_profiling(true)`. */
  streamUs: number
  /** CPU за кадр: сборка до `finish` (физика, отсечение, запись команд). */
  buildUs: number
  /** CPU за кадр: `finish`, `submit`, `present`. */
  submitUs: number
  /** Вершины и индексы, нарисованные в последнем кадре. */
  solidVertices: number
  solidIndices: number
  waterVertices: number
  waterIndices: number
  drawCalls: number
  /** Меши на GPU. */
  gpuKib: number
  /** Полная загрузка мира, мс; 0, пока идёт. */
  loadMs: number
}

export function parseStats(raw: ArrayLike<number>): WorldStats {
  const [
    columns,
    meshedColumns,
    meshedSections,
    queued,
    generateUs,
    meshUs,
    drawnSections,
    lightUs,
    editUs,
    lightKib,
    streamUs,
    buildUs,
    submitUs,
    solidVertices,
    solidIndices,
    waterVertices,
    waterIndices,
    drawCalls,
    gpuKib,
    loadMs,
  ] = Array.from(raw)
  return {
    columns,
    meshedColumns,
    meshedSections,
    queued,
    generateUs,
    meshUs,
    drawnSections,
    lightUs,
    editUs,
    lightKib,
    streamUs,
    buildUs,
    submitUs,
    solidVertices,
    solidIndices,
    waterVertices,
    waterIndices,
    drawCalls,
    gpuKib,
    loadMs,
  }
}
