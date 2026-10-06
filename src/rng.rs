//! Tiny deterministic PRNG. Hand-rolled on purpose: a crate upgrade must never
//! change what a share code produces.

pub struct Rng(u64);

impl Rng {
    pub fn from_str(s: &str) -> Self {
        // FNV-1a, then SplitMix does the mixing
        let mut h: u64 = 0xcbf29ce484222325;
        for b in s.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Rng(h)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    /// `pick` for a list of string literals, handing back the `&str` itself.
    pub fn pick_str(&mut self, xs: &[&'static str]) -> &'static str {
        xs[self.below(xs.len())] // same draw as `pick`, so output doesn't change
    }

    /// Picks index i with probability proportional to weights[i].
    pub fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut r = self.unit() * total;
        for (i, w) in weights.iter().enumerate() {
            if r < *w {
                return i;
            }
            r -= w;
        }
        weights.len() - 1
    }

    pub fn fork(&mut self, tag: &str) -> Rng {
        let mut r = Rng::from_str(tag);
        r.0 ^= self.next_u64();
        r
    }
}
