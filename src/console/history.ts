const LIMIT = 100

export class CommandHistory {
  private lines: string[] = []
  private cursor = 0
  private draft = ''

  push(line: string) {
    const duplicate = this.lines[this.lines.length - 1] === line
    if (line.trim() && !duplicate) {
      this.lines.push(line)
      if (this.lines.length > LIMIT) this.lines.shift()
    }
    this.cursor = this.lines.length
  }

  previous(current: string): string | undefined {
    if (this.cursor === 0) return undefined
    if (this.cursor === this.lines.length) this.draft = current
    this.cursor -= 1
    return this.lines[this.cursor]
  }

  next(): string | undefined {
    if (this.cursor >= this.lines.length) return undefined
    this.cursor += 1
    return this.cursor === this.lines.length ? this.draft : this.lines[this.cursor]
  }

  clear() {
    this.lines = []
    this.cursor = 0
  }

  reset() {
    this.cursor = this.lines.length
  }
}
