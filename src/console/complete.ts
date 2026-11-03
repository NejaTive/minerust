export interface Completion {
  start: number
  options: readonly string[]
}

export class TabCycler {
  private base = ''
  private options: readonly string[] = []
  private index = -1
  private applied: string | undefined

  next(line: string, complete: (line: string) => Completion): string | undefined {
    if (this.applied === undefined || line !== this.applied) {
      const { start, options } = complete(line)
      if (options.length === 0) return undefined
      this.base = line.slice(0, start)
      this.options = options
      this.index = -1
    }
    this.index = (this.index + 1) % this.options.length
    this.applied = this.base + this.options[this.index]
    return this.applied
  }

  reset() {
    this.applied = undefined
  }
}
