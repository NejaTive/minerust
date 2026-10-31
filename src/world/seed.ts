const I32_MIN = -(2 ** 31)
const I32_MAX = 2 ** 31 - 1

function javaStringHash(text: string): number {
  let hash = 0
  for (let i = 0; i < text.length; i++) hash = (Math.imul(hash, 31) + text.charCodeAt(i)) | 0
  return hash
}

export function seedFromInput(input: string, random: () => number): number {
  const text = input.trim()
  if (text === '') return random()
  if (/^-?\d+$/.test(text)) {
    const value = Number(text)
    if (value >= I32_MIN && value <= I32_MAX) return value
  }
  return javaStringHash(text)
}
