// sha("{prefix}:{solution}") has {complexity} leading zero bits
export async function solvePow(prefix: string, complexity: number, updateProgress: (progress: { percents: number, eta: number }) => void): Promise<string> {
    let current = 0;
    let startTime = Date.now();
    const targetBits = complexity;
    const expectedTotal = 2 ** complexity;
    let percents = 0;

    const updateProgressInterval = () => {
        const elapsed = (Date.now() - startTime) / 1000;
        const rate = current / (elapsed || 1);
        const remaining = Math.max(0, expectedTotal - current);
        const eta = Math.ceil(remaining / (rate || 1));
        percents = Math.min(100, (current / expectedTotal) * 100);

        updateProgress({ percents, eta });
    };

    const progressInterval = setInterval(updateProgressInterval, 100);

    try {
        const hashString = async (str: string): Promise<string> => {
            const encoder = new TextEncoder();
            const data = encoder.encode(str);
            const hashBuffer = await crypto.subtle.digest('SHA-256', data);
            const hashArray = Array.from(new Uint8Array(hashBuffer));
            return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
        };

        const countLeadingZeroBits = (hexHash: string): number => {
            let count = 0;

            for (let i = 0; i < hexHash.length; i++) {
                const nibble = parseInt(hexHash[i], 16);

                if (nibble === 0) {
                    count += 4;
                } else {
                    for (let j = 3; j >= 0; j--) {
                        if ((nibble & (1 << j)) === 0) {
                            count++;
                        } else {
                            return count;
                        }
                    }
                }
            }

            return count;
        };

        let solution = 0;
        while (true) {
            const candidate = `${prefix}:${solution}`;
            const hash = await hashString(candidate);
            const leadingZeros = countLeadingZeroBits(hash);

            current += 1;

            if (current % 1000 === 0) {
                updateProgressInterval();
            }

            if (leadingZeros >= targetBits) {
                clearInterval(progressInterval);
                updateProgressInterval();
                return solution.toString();
            }

            solution += 1;

            if (solution % 1000 === 0) {
                await new Promise(resolve => setTimeout(resolve, 0));
            }
        }
    } finally {
        clearInterval(progressInterval);
    }
}