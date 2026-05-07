const DATA_CODEWORDS_L = [0, 19, 34, 55, 80, 108, 136]
const ECC_CODEWORDS_PER_BLOCK_L = [0, 7, 10, 15, 20, 26, 18]
const BLOCK_COUNT_L = [0, 1, 1, 1, 1, 1, 2]
const ALIGNMENT_POSITIONS: Record<number, number[]> = {
  1: [],
  2: [6, 18],
  3: [6, 22],
  4: [6, 26],
  5: [6, 30],
  6: [6, 34],
}

type ModuleValue = boolean | null

interface QrMatrix {
  size: number
  modules: ModuleValue[][]
  functionModules: boolean[][]
}

export function createQrCodeSvgDataUrl(text: string): string {
  const normalized = text.trim()
  if (!normalized) return ''

  const bytes = Array.from(new TextEncoder().encode(normalized))
  const version = chooseVersion(bytes.length)
  const codewords = buildCodewords(bytes, version)
  const base = createBaseMatrix(version)
  drawCodewords(base, codewords)

  let bestModules: boolean[][] | null = null
  let bestPenalty = Number.POSITIVE_INFINITY
  for (let mask = 0; mask < 8; mask += 1) {
    const candidate = cloneModules(base.modules)
    applyMask(candidate, base.functionModules, mask)
    drawFormatBits(candidate, base.functionModules, mask)
    const penalty = scorePenalty(candidate)
    if (penalty < bestPenalty) {
      bestPenalty = penalty
      bestModules = candidate
    }
  }

  if (!bestModules) return ''
  return renderSvgDataUrl(bestModules)
}

function chooseVersion(byteLength: number): number {
  for (let version = 1; version <= 6; version += 1) {
    const bitLength = 4 + 8 + byteLength * 8
    if (bitLength <= DATA_CODEWORDS_L[version] * 8) return version
  }
  throw new Error('QR content is too long for the local Weixin QR renderer')
}

function buildCodewords(bytes: number[], version: number): number[] {
  const dataCodewords = DATA_CODEWORDS_L[version]
  const bits: number[] = []
  appendBits(bits, 0b0100, 4)
  appendBits(bits, bytes.length, 8)
  for (const byte of bytes) appendBits(bits, byte, 8)

  const capacityBits = dataCodewords * 8
  appendBits(bits, 0, Math.min(4, capacityBits - bits.length))
  while (bits.length % 8 !== 0) bits.push(0)

  const data = bitsToBytes(bits)
  for (let pad = 0xec; data.length < dataCodewords; pad = pad === 0xec ? 0x11 : 0xec) {
    data.push(pad)
  }

  const blockCount = BLOCK_COUNT_L[version]
  const blockSize = dataCodewords / blockCount
  const eccSize = ECC_CODEWORDS_PER_BLOCK_L[version]
  const blocks = Array.from({ length: blockCount }, (_, index) => {
    const start = index * blockSize
    const blockData = data.slice(start, start + blockSize)
    return {
      data: blockData,
      ecc: reedSolomonRemainder(blockData, eccSize),
    }
  })

  const result: number[] = []
  for (let i = 0; i < blockSize; i += 1) {
    for (const block of blocks) result.push(block.data[i])
  }
  for (let i = 0; i < eccSize; i += 1) {
    for (const block of blocks) result.push(block.ecc[i])
  }
  return result
}

function createBaseMatrix(version: number): QrMatrix {
  const size = version * 4 + 17
  const modules = Array.from({ length: size }, () => Array<ModuleValue>(size).fill(null))
  const functionModules = Array.from({ length: size }, () => Array<boolean>(size).fill(false))
  const matrix = { size, modules, functionModules }

  drawFinderPattern(matrix, 3, 3)
  drawFinderPattern(matrix, size - 4, 3)
  drawFinderPattern(matrix, 3, size - 4)
  drawAlignmentPatterns(matrix, version)
  drawTimingPatterns(matrix)
  reserveFormatAreas(matrix)
  setFunctionModule(matrix, 8, size - 8, true)
  return matrix
}

function drawFinderPattern(matrix: QrMatrix, centerX: number, centerY: number) {
  for (let dy = -4; dy <= 4; dy += 1) {
    for (let dx = -4; dx <= 4; dx += 1) {
      const x = centerX + dx
      const y = centerY + dy
      if (x < 0 || y < 0 || x >= matrix.size || y >= matrix.size) continue
      const dist = Math.max(Math.abs(dx), Math.abs(dy))
      setFunctionModule(matrix, x, y, dist !== 2 && dist !== 4)
    }
  }
}

function drawAlignmentPatterns(matrix: QrMatrix, version: number) {
  const positions = ALIGNMENT_POSITIONS[version] || []
  for (const y of positions) {
    for (const x of positions) {
      const overlapsFinder =
        (x <= 8 && y <= 8) ||
        (x >= matrix.size - 9 && y <= 8) ||
        (x <= 8 && y >= matrix.size - 9)
      if (overlapsFinder) continue
      for (let dy = -2; dy <= 2; dy += 1) {
        for (let dx = -2; dx <= 2; dx += 1) {
          setFunctionModule(matrix, x + dx, y + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1)
        }
      }
    }
  }
}

function drawTimingPatterns(matrix: QrMatrix) {
  for (let i = 8; i < matrix.size - 8; i += 1) {
    const value = i % 2 === 0
    setFunctionModule(matrix, 6, i, value)
    setFunctionModule(matrix, i, 6, value)
  }
}

function reserveFormatAreas(matrix: QrMatrix) {
  for (let i = 0; i < 9; i += 1) {
    if (i !== 6) {
      setFunctionModule(matrix, 8, i, false)
      setFunctionModule(matrix, i, 8, false)
    }
  }
  for (let i = 0; i < 8; i += 1) {
    setFunctionModule(matrix, matrix.size - 1 - i, 8, false)
    setFunctionModule(matrix, 8, matrix.size - 1 - i, false)
  }
}

function drawCodewords(matrix: QrMatrix, codewords: number[]) {
  let bitIndex = 0
  let upward = true
  for (let right = matrix.size - 1; right >= 1; right -= 2) {
    if (right === 6) right -= 1
    for (let vert = 0; vert < matrix.size; vert += 1) {
      const y = upward ? matrix.size - 1 - vert : vert
      for (let offset = 0; offset < 2; offset += 1) {
        const x = right - offset
        if (matrix.functionModules[y][x]) continue
        const byte = codewords[Math.floor(bitIndex / 8)] || 0
        matrix.modules[y][x] = ((byte >>> (7 - (bitIndex % 8))) & 1) !== 0
        bitIndex += 1
      }
    }
    upward = !upward
  }
}

function applyMask(modules: ModuleValue[][], functionModules: boolean[][], mask: number) {
  for (let y = 0; y < modules.length; y += 1) {
    for (let x = 0; x < modules.length; x += 1) {
      if (functionModules[y][x]) continue
      if (maskBit(mask, x, y)) modules[y][x] = !modules[y][x]
    }
  }
}

function drawFormatBits(modules: ModuleValue[][], functionModules: boolean[][], mask: number) {
  const size = modules.length
  const bits = formatBits(mask)
  for (let i = 0; i <= 5; i += 1) setModule(modules, functionModules, 8, i, getBit(bits, i))
  setModule(modules, functionModules, 8, 7, getBit(bits, 6))
  setModule(modules, functionModules, 8, 8, getBit(bits, 7))
  setModule(modules, functionModules, 7, 8, getBit(bits, 8))
  for (let i = 9; i < 15; i += 1) setModule(modules, functionModules, 14 - i, 8, getBit(bits, i))
  for (let i = 0; i < 8; i += 1) setModule(modules, functionModules, size - 1 - i, 8, getBit(bits, i))
  for (let i = 8; i < 15; i += 1) setModule(modules, functionModules, 8, size - 15 + i, getBit(bits, i))
  setModule(modules, functionModules, 8, size - 8, true)
}

function formatBits(mask: number): number {
  const data = (1 << 3) | mask
  let bits = data << 10
  for (let i = 14; i >= 10; i -= 1) {
    if (((bits >>> i) & 1) !== 0) bits ^= 0x537 << (i - 10)
  }
  return ((data << 10) | bits) ^ 0x5412
}

function reedSolomonRemainder(data: number[], degree: number): number[] {
  const generator = reedSolomonGenerator(degree)
  const result = [...data, ...Array<number>(degree).fill(0)]
  for (let i = 0; i < data.length; i += 1) {
    const factor = result[i]
    if (factor === 0) continue
    for (let j = 0; j < generator.length; j += 1) {
      result[i + j] ^= gfMultiply(generator[j], factor)
    }
  }
  return result.slice(data.length)
}

function reedSolomonGenerator(degree: number): number[] {
  let result = [1]
  for (let i = 0; i < degree; i += 1) {
    result = polyMultiply(result, [1, gfPow(i)])
  }
  return result
}

function polyMultiply(a: number[], b: number[]): number[] {
  const result = Array<number>(a.length + b.length - 1).fill(0)
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) {
      result[i + j] ^= gfMultiply(a[i], b[j])
    }
  }
  return result
}

function gfPow(power: number): number {
  let result = 1
  for (let i = 0; i < power; i += 1) result = gfMultiply(result, 2)
  return result
}

function gfMultiply(a: number, b: number): number {
  let result = 0
  let x = a
  let y = b
  while (y !== 0) {
    if ((y & 1) !== 0) result ^= x
    x <<= 1
    if ((x & 0x100) !== 0) x ^= 0x11d
    y >>>= 1
  }
  return result
}

function scorePenalty(modules: ModuleValue[][]): number {
  const size = modules.length
  let penalty = 0

  for (let y = 0; y < size; y += 1) penalty += scoreRunPenalty(modules[y].map(Boolean))
  for (let x = 0; x < size; x += 1) penalty += scoreRunPenalty(modules.map((row) => Boolean(row[x])))

  for (let y = 0; y < size - 1; y += 1) {
    for (let x = 0; x < size - 1; x += 1) {
      const color = modules[y][x]
      if (color === modules[y][x + 1] && color === modules[y + 1][x] && color === modules[y + 1][x + 1]) {
        penalty += 3
      }
    }
  }

  const pattern = [true, false, true, true, true, false, true, false, false, false, false]
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x <= size - pattern.length; x += 1) {
      if (matchesPattern(modules[y], x, pattern) || matchesPattern(modules[y], x, [...pattern].reverse())) penalty += 40
    }
  }
  for (let x = 0; x < size; x += 1) {
    const col = modules.map((row) => row[x])
    for (let y = 0; y <= size - pattern.length; y += 1) {
      if (matchesPattern(col, y, pattern) || matchesPattern(col, y, [...pattern].reverse())) penalty += 40
    }
  }

  const dark = modules.flat().filter(Boolean).length
  const total = size * size
  const k = Math.floor(Math.abs((dark * 20) / total - 10))
  penalty += k * 10
  return penalty
}

function scoreRunPenalty(values: boolean[]): number {
  let penalty = 0
  let runColor = values[0]
  let runLength = 1
  for (let i = 1; i < values.length; i += 1) {
    if (values[i] === runColor) {
      runLength += 1
      continue
    }
    if (runLength >= 5) penalty += runLength - 2
    runColor = values[i]
    runLength = 1
  }
  if (runLength >= 5) penalty += runLength - 2
  return penalty
}

function matchesPattern(values: ModuleValue[], offset: number, pattern: boolean[]): boolean {
  return pattern.every((value, index) => Boolean(values[offset + index]) === value)
}

function renderSvgDataUrl(modules: boolean[][]): string {
  const quiet = 4
  const size = modules.length
  const viewSize = size + quiet * 2
  const path = modules
    .flatMap((row, y) =>
      row
        .map((value, x) => (value ? `M${x + quiet},${y + quiet}h1v1h-1z` : ''))
        .filter(Boolean),
    )
    .join('')
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${viewSize} ${viewSize}" shape-rendering="crispEdges"><path fill="#fff" d="M0 0h${viewSize}v${viewSize}H0z"/><path fill="#111827" d="${path}"/></svg>`
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

function appendBits(bits: number[], value: number, length: number) {
  for (let i = length - 1; i >= 0; i -= 1) bits.push((value >>> i) & 1)
}

function bitsToBytes(bits: number[]): number[] {
  const result: number[] = []
  for (let i = 0; i < bits.length; i += 8) {
    let byte = 0
    for (let j = 0; j < 8; j += 1) byte = (byte << 1) | bits[i + j]
    result.push(byte)
  }
  return result
}

function setFunctionModule(matrix: QrMatrix, x: number, y: number, value: boolean) {
  matrix.modules[y][x] = value
  matrix.functionModules[y][x] = true
}

function setModule(modules: ModuleValue[][], functionModules: boolean[][], x: number, y: number, value: boolean) {
  modules[y][x] = value
  functionModules[y][x] = true
}

function cloneModules(modules: ModuleValue[][]): boolean[][] {
  return modules.map((row) => row.map(Boolean))
}

function getBit(value: number, index: number): boolean {
  return ((value >>> index) & 1) !== 0
}

function maskBit(mask: number, x: number, y: number): boolean {
  switch (mask) {
    case 0:
      return (x + y) % 2 === 0
    case 1:
      return y % 2 === 0
    case 2:
      return x % 3 === 0
    case 3:
      return (x + y) % 3 === 0
    case 4:
      return (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0
    case 5:
      return ((x * y) % 2) + ((x * y) % 3) === 0
    case 6:
      return (((x * y) % 2) + ((x * y) % 3)) % 2 === 0
    case 7:
      return (((x + y) % 2) + ((x * y) % 3)) % 2 === 0
    default:
      return false
  }
}
