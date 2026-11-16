//! Deterministic fast hasher (FNV-1a, 64-bit) for hot signature paths
//!
//! Phase E: the tableau hot paths (blocking signature keys, signature caches,
//! clause-check result cache) hashed with `std`'s `SipHash` (`DefaultHasher` /
//! `RandomState`) even though they only ever need a *fast, stable* bucket
//! key — never a cryptographic-quality hash. FNV-1a is a fixed, seedless
//! mixing function: one multiply-xor per byte, no key schedule, no random
//! seed, and it implements [`Hasher`], so it drops straight into every
//! `HashMap`/`HashSet`/`LruCache` that accepts a `BuildHasher`.
//!
//! # Determinism
//!
//! * **No random seed.** Unlike `RandomState` (randomly seeded *per process*),
//!   [`FastHasher`] always starts from the same FNV-1a offset basis, so the
//!   same input bytes produce the same `u64` in every process, every run.
//! * **Endian-normalised integer writes.** `Hash`'s default `write_u64` etc.
//!   delegate to `write(&n.to_ne_bytes())`, which is *native-endian*; this
//!   hasher overrides every integer/`usize`/length-prefix write with
//!   **little-endian** bytes so hashes are also stable across architectures.
//! * The hash *values* differ from `DefaultHasher`/`RandomState` outputs —
//!   they are **internal cache keys only**. Every consumer of these hashes
//!   re-verifies positives with real equality (`ConceptLabel` equality,
//!   `CachedCheckResult` payloads), so a collision can never change a
//!   decision: a mismatch only proves inequality, a match is always
//!   re-checked structurally.
//!
//! `DefaultHasher` stays in use everywhere outside the Phase E hot paths
//! (this module deliberately changes no other hashing site).

use std::hash::{BuildHasherDefault, Hash, Hasher};

/// FNV-1a 64-bit offset basis (`0xcbf29ce484222325`).
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime (`2^40 + 2^8 + 0xb3 = 1_099_511_628_211`).
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A deterministic, seedless, non-cryptographic 64-bit hasher (FNV-1a).
///
/// Intended for *internal* cache keys on hot paths — see the module docs for
/// the determinism contract. Not for anything security-relevant.
#[derive(Debug, Clone, Copy)]
pub struct FastHasher {
    hash: u64,
}

impl FastHasher {
    /// Create a hasher positioned at the FNV-1a offset basis.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            hash: FNV_OFFSET_BASIS,
        }
    }
}

impl Default for FastHasher {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FastHasher {
    /// FNV-1a over raw bytes: xor each byte, then multiply by the prime.
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.hash;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        self.hash = hash;
    }

    // ── Endian-normalised integer writes (cross-platform determinism) ──────
    // `Hash`'s default impls route integers through `write(&n.to_ne_bytes())`;
    // overriding them with LE bytes keeps the digest architecture-independent.

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8);
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// `BuildHasher` factory for [`FastHasher`] — the type alias to use as the
/// third parameter of `HashMap`/`HashSet` (and `LruCache`) on Phase E paths.
pub type FastBuildHasher = BuildHasherDefault<FastHasher>;

/// Convenience: hash any [`Hash`] value with [`FastHasher`] in one call.
#[inline]
#[must_use]
pub fn fast_hash<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = FastHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn fast_hash_is_deterministic_across_instances() {
        // No random seed: two fresh hashers over the same input agree.
        assert_eq!(
            fast_hash(&"http://example.org/A"),
            fast_hash(&"http://example.org/A")
        );
        assert_ne!(
            fast_hash(&"http://example.org/A"),
            fast_hash(&"http://example.org/B")
        );
    }

    #[test]
    fn integer_writes_are_little_endian_normalized() {
        // A u64 must hash identically regardless of the platform's native
        // endianness — pin the LE byte order explicitly.
        let mut le = FastHasher::new();
        le.write(&0x0102_0304_0506_0708u64.to_le_bytes());
        let mut via_trait = FastHasher::new();
        0x0102_0304_0506_0708u64.hash(&mut via_trait);
        assert_eq!(le.finish(), via_trait.finish());
    }

    #[test]
    fn fast_hasher_differs_from_default_hasher_but_is_stable() {
        let value = "http://example.org/A";
        let mut fast = FastHasher::new();
        value.hash(&mut fast);
        let mut slow = DefaultHasher::new();
        value.hash(&mut slow);
        // Different algorithms ⇒ (almost surely) different digests; the point
        // is that each is internally consistent, never that they agree.
        assert_ne!(fast.finish(), slow.finish());
        assert_eq!(fast_hash(value), fast_hash(value));
    }

    #[test]
    fn build_hasher_default_works_in_hashmap() {
        let mut map: HashMap<&str, u64, FastBuildHasher> = HashMap::default();
        map.insert("a", 1);
        assert_eq!(map.get("a"), Some(&1));
    }
}
