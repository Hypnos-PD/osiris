use core::num::Wrapping;

/// Mersenne Twister 19937 implementation matching C++ std::mt19937 output.
/// Implements the standard initialization and gen_u32 output.
pub struct Mt19937 {
    mt: [u32; 624],
    mti: usize,
}

impl Mt19937 {
    const N: usize = 624;
    const M: usize = 397;
    const MATRIX_A: u32 = 0x9908b0df;
    const UPPER_MASK: u32 = 0x80000000;
    const LOWER_MASK: u32 = 0x7fffffff;

    pub fn new(seed: u32) -> Self {
        let mut mt = [0u32; Self::N];
        mt[0] = seed;
        for i in 1..Self::N {
            // mt[i] = (1812433253 * (mt[i-1] xor (mt[i-1] >> 30)) + i)
            // use wrapping to replicate behaviour
            let prev = Wrapping(mt[i - 1]);
            let mut val = prev ^ (prev >> 30);
            val = val * Wrapping(1812433253u32);
            val += Wrapping(i as u32);
            mt[i] = val.0;
        }
        Mt19937 { mt, mti: Self::N }
    }

    pub fn gen_u32(&mut self) -> u32 {
        if self.mti >= Self::N {
            // generate N words at one time
            let mut kk = 0usize;
            while kk < Self::N - Self::M {
                let y = (self.mt[kk] & Self::UPPER_MASK) | (self.mt[kk + 1] & Self::LOWER_MASK);
                self.mt[kk] = self.mt[kk + Self::M] ^ (y >> 1) ^ (if (y & 1) != 0 { Self::MATRIX_A } else { 0 });
                kk += 1;
            }
            while kk < Self::N - 1 {
                let y = (self.mt[kk] & Self::UPPER_MASK) | (self.mt[kk + 1] & Self::LOWER_MASK);
                // compute index accounting for wrap-around (kk + M - N), done safely using isize
                let idx = (kk as isize + Self::M as isize - Self::N as isize) as usize;
                self.mt[kk] = self.mt[idx] ^ (y >> 1) ^ (if (y & 1) != 0 { Self::MATRIX_A } else { 0 });
                kk += 1;
            }
            let y = (self.mt[Self::N - 1] & Self::UPPER_MASK) | (self.mt[0] & Self::LOWER_MASK);
            self.mt[Self::N - 1] = self.mt[Self::M - 1] ^ (y >> 1) ^ (if (y & 1) != 0 { Self::MATRIX_A } else { 0 });
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;

        // Tempering
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }

    pub fn get_next_integer(&mut self, min: u32, max: u32) -> u32 {
        let range = max - min + 1;
        let bound = (u32::MAX as u64).wrapping_add(1).wrapping_sub(range as u64) % (range as u64);
        let mut x = self.gen_u32();
        while (x as u64) < bound {
            x = self.gen_u32();
        }
        let result = min + (x % range);
        println!("get_next_integer({}, {}) -> range={}, bound={}, x={}, result={}", min, max, range, bound, x, result);
        result
    }

    pub fn shuffle_vector<T>(&mut self, v: &mut Vec<T>, first: usize, last: usize) {
        let last = if last > v.len() { v.len() } else { last };
        println!("Shuffling vector from {} to {}", first, last);
        for i in first..(last.saturating_sub(1)) {
            let r = self.get_next_integer(i as u32, (last - 1) as u32) as usize;
            println!("  Swap {} with {} (range: {} to {})", i, r, i, last - 1);
            v.swap(i, r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mt19937;

    #[test]
    fn mt19937_matches_reference() {
        // Known sequence for a seed 5489 from reference implementation
        // First 10 outputs of MT19937 with seed 5489 (from original RFC):
        // 3499211612, 581869302, 3890346734, 3586334585, 545404204,
        // 4161255391, 3922919429, 949333985, 2715962298, 1323567403
        let mut mt = Mt19937::new(5489);
        let expected: [u32; 10] = [
            3499211612, 581869302, 3890346734, 3586334585, 545404204,
            4161255391, 3922919429, 949333985, 2715962298, 1323567403,
        ];
        for &e in expected.iter() {
            assert_eq!(mt.gen_u32(), e);
        }
    }

    #[test]
    fn test_mt19937_standard_values() {
        // Check known MT19937 outputs for seed 42
        let mut mt = Mt19937::new(42);
        let first = mt.gen_u32();
        let second = mt.gen_u32();
        assert_eq!(first, 1608637542);
        assert_eq!(second, 3421126067);
    }

    #[test]
    fn test_shuffle_algorithm() {
        let seed: u32 = 42;
        let mut mt = Mt19937::new(seed);
        
        // Test shuffle with same deck as in the test
        let mut deck = vec![100, 200, 300, 400, 500, 600, 700];
        let original = deck.clone();
        println!("Before shuffle: {:?}", deck);
        let deck_len = deck.len();
        mt.shuffle_vector(&mut deck, 0, deck_len);
        println!("After shuffle: {:?}", deck);
        
        // The deck should be shuffled
        assert_ne!(deck, original);
    }
}
