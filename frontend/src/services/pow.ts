// sha("{prefix}:{solution}") must have the required number of leading zero bits.
export interface PowProgressUpdate {
  percents: number
  eta: number
  /** Хешей в секунду, используется только для отображения прогресса. */
  hashRatePerSec: number
}

export async function solvePow(
  prefix: string,
  complexity: number,
  updateProgress: (progress: PowProgressUpdate) => void,
): Promise<string> {
  let current = 0
  const startTime = Date.now()
  const targetBits = complexity
  const expectedTotal = 2 ** complexity

  const updateProgressInterval = () => {
    const elapsed = (Date.now() - startTime) / 1000
    const rate = current / (elapsed || 1)
    const remaining = Math.max(0, expectedTotal - current)
    const eta = Math.ceil(remaining / (rate || 1))
    const percents = Math.min(100, (current / expectedTotal) * 100)

    updateProgress({ percents, eta, hashRatePerSec: rate })
  }

  const progressInterval = setInterval(updateProgressInterval, 100)

  try {
    const hashString = async (value: string): Promise<string> => {
      const encoder = new TextEncoder()
      const data = encoder.encode(value)
      const hashBuffer = await crypto.subtle.digest('SHA-256', data)
      const hashArray = Array.from(new Uint8Array(hashBuffer))
      return hashArray.map((byte) => byte.toString(16).padStart(2, '0')).join('')
    }

    const countLeadingZeroBits = (hexHash: string): number => {
      let count = 0

      for (let index = 0; index < hexHash.length; index += 1) {
        const nibble = parseInt(hexHash[index], 16)

        if (nibble === 0) {
          count += 4
        } else {
          for (let bit = 3; bit >= 0; bit -= 1) {
            if ((nibble & (1 << bit)) === 0) {
              count += 1
            } else {
              return count
            }
          }
        }
      }

      return count
    }

    let solution = 0
    while (true) {
      const candidate = `${prefix}:${solution}`
      const hash = await hashString(candidate)
      const leadingZeros = countLeadingZeroBits(hash)

      current += 1

      if (current % 1000 === 0) {
        updateProgressInterval()
      }

      if (leadingZeros >= targetBits) {
        clearInterval(progressInterval)
        updateProgressInterval()
        return solution.toString()
      }

      solution += 1

      if (solution % 1000 === 0) {
        await new Promise((resolve) => setTimeout(resolve, 0))
      }
    }
  } finally {
    clearInterval(progressInterval)
  }
}
