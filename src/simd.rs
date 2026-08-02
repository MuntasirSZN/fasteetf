// ─────────────────────────────────────────────────────────────────────────────
// SIMD optimizations for atom comparison and binary operations
//
// Comprehensive SIMD support:
// - x86_64: SSE2, SSE3, SSSE3, SSE4.1, SSE4.2, AVX, AVX2, AVX-512
// - aarch64: NEON (ASIMD) - always available on aarch64
//
// Uses `core::arch` with runtime CPU feature detection via `cpufeatures`.
// Falls back to scalar code when SIMD is not available.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
#[cfg(target_arch = "x86_64")]
use cpufeatures::new;

// x86_64 CPU feature detection
#[cfg(target_arch = "x86_64")]
new!(cpuid_sse2, "sse2");
#[cfg(target_arch = "x86_64")]
new!(cpuid_sse3, "sse3");
#[cfg(target_arch = "x86_64")]
new!(cpuid_ssse3, "ssse3");
#[cfg(target_arch = "x86_64")]
new!(cpuid_sse4_1, "sse4.1");
#[cfg(target_arch = "x86_64")]
new!(cpuid_sse4_2, "sse4.2");
#[cfg(target_arch = "x86_64")]
new!(cpuid_avx, "avx");
#[cfg(target_arch = "x86_64")]
new!(cpuid_avx2, "avx2");
#[cfg(target_arch = "x86_64")]
new!(cpuid_avx512f, "avx512f");

/// Compare two byte slices for equality using the best available SIMD (x86_64).
///
/// Every full block of the selected tier width is compared; any leftover
/// tail is checked with a scalar slice comparison.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub(crate) fn simd_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let len = a.len();
    if len == 0 {
        return true;
    }

    let mut i = 0usize;
    unsafe {
        // AVX-512: 64 bytes at a time
        if cpuid_avx512f::get() {
            while i + 64 <= len {
                let a_simd = _mm512_loadu_si512(a.as_ptr().add(i) as *const __m512i);
                let b_simd = _mm512_loadu_si512(b.as_ptr().add(i) as *const __m512i);
                if _mm512_cmpeq_epi8_mask(a_simd, b_simd) != u64::MAX {
                    return false;
                }
                i += 64;
            }
            return a[i..] == b[i..];
        }
        // AVX2: 32 bytes at a time
        if cpuid_avx2::get() {
            while i + 32 <= len {
                let a_simd = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
                let b_simd = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);
                if _mm256_movemask_epi8(_mm256_cmpeq_epi8(a_simd, b_simd)) != -1 {
                    return false;
                }
                i += 32;
            }
            return a[i..] == b[i..];
        }
        // SSE2+: 16 bytes at a time
        if cpuid_sse2::get() {
            while i + 16 <= len {
                let a_simd = _mm_loadu_si128(a.as_ptr().add(i) as *const __m128i);
                let b_simd = _mm_loadu_si128(b.as_ptr().add(i) as *const __m128i);
                if _mm_movemask_epi8(_mm_cmpeq_epi8(a_simd, b_simd)) != 0xFFFF {
                    return false;
                }
                i += 16;
            }
            return a[i..] == b[i..];
        }
    }
    // Fall back to scalar
    a == b
}

/// Compare two byte slices for equality using NEON (aarch64).
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub(crate) fn simd_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let len = a.len();
    if len == 0 {
        return true;
    }

    // NEON: 16 bytes at a time, then the scalar tail
    let mut i = 0usize;
    unsafe {
        while i + 16 <= len {
            let a_simd = vld1q_u8(a.as_ptr().add(i));
            let b_simd = vld1q_u8(b.as_ptr().add(i));
            if vminvq_u8(vceqq_u8(a_simd, b_simd)) != 255 {
                return false;
            }
            i += 16;
        }
    }
    &a[i..] == &b[i..]
}

/// Copy bytes using SIMD (x86_64).
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub(crate) unsafe fn simd_copy(dst: *mut u8, src: *const u8, len: usize) {
    unsafe {
        let mut i = 0usize;

        // AVX-512: 64 bytes at a time
        if cpuid_avx512f::get() {
            while i + 64 <= len {
                let src_simd = _mm512_loadu_si512(src.add(i) as *const __m512i);
                _mm512_storeu_si512(dst.add(i) as *mut __m512i, src_simd);
                i += 64;
            }
        }
        // AVX2: 32 bytes at a time
        else if cpuid_avx2::get() {
            while i + 32 <= len {
                let src_simd = _mm256_loadu_si256(src.add(i) as *const __m256i);
                _mm256_storeu_si256(dst.add(i) as *mut __m256i, src_simd);
                i += 32;
            }
        }
        // SSE2: 16 bytes at a time
        else if cpuid_sse2::get() {
            while i + 16 <= len {
                let src_simd = _mm_loadu_si128(src.add(i) as *const __m128i);
                _mm_storeu_si128(dst.add(i) as *mut __m128i, src_simd);
                i += 16;
            }
        }

        // Copy remaining bytes
        if i < len {
            core::ptr::copy_nonoverlapping(src.add(i), dst.add(i), len - i);
        }
    }
}

/// Copy bytes using NEON (aarch64).
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub(crate) unsafe fn simd_copy(dst: *mut u8, src: *const u8, len: usize) {
    unsafe {
        let mut i = 0usize;

        // NEON: 16 bytes at a time
        while i + 16 <= len {
            let src_simd = vld1q_u8(src.add(i));
            vst1q_u8(dst.add(i), src_simd);
            i += 16;
        }

        // Copy remaining bytes
        if i < len {
            core::ptr::copy_nonoverlapping(src.add(i), dst.add(i), len - i);
        }
    }
}

/// Scalar fallback for simd_eq (non-x86_64, non-aarch64).
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
pub(crate) fn simd_eq(a: &[u8], b: &[u8]) -> bool {
    a == b
}

/// Scalar fallback for simd_copy (non-x86_64, non-aarch64).
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
pub(crate) unsafe fn simd_copy(dst: *mut u8, src: *const u8, len: usize) {
    core::ptr::copy_nonoverlapping(src, dst, len);
}

// ── Unit tests ──────────────────────────────────────────────────────────────
//
// Buffer lengths are chosen to walk every SIMD tier:
//   < 16 bytes  → scalar fallback
//   16..32      → SSE2 path
//   32..64      → AVX2 path
//   >= 64       → AVX-512 path (only exercised on AVX-512-capable hardware)

#[cfg(test)]
mod tests {
    use super::{simd_copy, simd_eq};

    #[test]
    fn eq_empty_and_mismatched_lengths() {
        assert!(simd_eq(&[], &[]));
        assert!(!simd_eq(&[], &[1]));
        assert!(!simd_eq(&[1, 2], &[1, 2, 3]));
    }

    #[test]
    fn eq_short_scalar_path() {
        assert!(simd_eq(b"abcdefgh", b"abcdefgh"));
        assert!(!simd_eq(b"abcdefgh", b"abcdefgi"));
    }

    #[test]
    fn eq_16_byte_sse2_path() {
        assert!(simd_eq(b"abcdefghijklmnop", b"abcdefghijklmnop"));
        assert!(!simd_eq(b"abcdefghijklmnop", b"abcdefghijklmnoq"));
    }

    #[test]
    fn eq_32_byte_avx2_path() {
        assert!(simd_eq(
            b"abcdefghijklmnopqrstuvwxyz012345",
            b"abcdefghijklmnopqrstuvwxyz012345"
        ));
        assert!(!simd_eq(
            b"abcdefghijklmnopqrstuvwxyz012345",
            b"abcdefghijklmnopqrstuvwxyz012346"
        ));
    }

    #[test]
    fn eq_64_byte_avx512_path() {
        // Only reaches the AVX-512 intrinsics on AVX-512-capable machines.
        let a = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ab";
        let b = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ab";
        assert_eq!(a.len(), 64);
        assert!(simd_eq(a, b));
        assert!(!simd_eq(
            a,
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ac"
        ));
    }

    #[test]
    fn eq_differs_in_tail_after_simd_block() {
        // Regression: lengths that are not exact multiples of the SIMD tier
        // width used to compare only the first block, so a difference in the
        // tail was silently ignored.
        // SSE2 tier: 17 bytes, differs at byte 16.
        assert!(!simd_eq(b"abcdefghijklmnopqX", b"abcdefghijklmnopqY"));
        // AVX2 tier: 33 bytes, differs at byte 32.
        assert!(!simd_eq(
            b"abcdefghijklmnopqrstuvwxyz012345X",
            b"abcdefghijklmnopqrstuvwxyz012345Y"
        ));
        // AVX-512 tier: 65 bytes, differs at byte 64.
        assert!(!simd_eq(
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abX",
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abY"
        ));
        // Same length, equal tails: must still compare equal.
        assert!(simd_eq(
            b"abcdefghijklmnopqrstuvwxyz012345X",
            b"abcdefghijklmnopqrstuvwxyz012345X"
        ));
    }

    #[test]
    fn copy_all_tiers_and_remainder() {
        let src = b"the quick brown fox jumps over the lazy dog 0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        assert!(src.len() >= 100);
        for len in [0usize, 1, 8, 15, 16, 17, 31, 32, 33, 47, 48, 64, 100] {
            let mut dst = [0u8; 128];
            // SAFETY: dst is 128 bytes, len <= 100, regions are disjoint.
            unsafe { simd_copy(dst.as_mut_ptr(), src.as_ptr(), len) };
            assert_eq!(&dst[..len], &src[..len], "len = {len}");
        }
    }
}
