/// Tiny deterministic xorshift64* PRNG. Zero dependencies and works on wasm
/// (no `getrandom`/js glue needed); the seed comes from the caller so games
/// and tests are fully reproducible.
#[derive(Clone, Debug)]
pub struct Xor {
    state: u64,
}

impl Xor {
    pub fn new(seed: u64) -> Self {
        Xor {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform value in `0..n` (0 if `n == 0`).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as u32
        }
    }
}
