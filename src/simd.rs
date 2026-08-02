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

    // AVX-512: 64 bytes at a time
    if len >= 64 && cpuid_avx512f::get() {
        unsafe {
            let a_simd = _mm512_loadu_si512(a.as_ptr() as *const __m512i);
            let b_simd = _mm512_loadu_si512(b.as_ptr() as *const __m512i);
            _mm512_cmpeq_epi8_mask(a_simd, b_simd) == 0xFFFF
        }
    }
    // AVX2: 32 bytes at a time
    else if len >= 32 && cpuid_avx2::get() {
        unsafe {
            let a_simd = _mm256_loadu_si256(a.as_ptr() as *const __m256i);
            let b_simd = _mm256_loadu_si256(b.as_ptr() as *const __m256i);
            _mm256_movemask_epi8(_mm256_cmpeq_epi8(a_simd, b_simd)) == -1
        }
    }
    // SSE2+: 16 bytes at a time
    else if len >= 16 && cpuid_sse2::get() {
        unsafe {
            let a_simd = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_simd = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            _mm_movemask_epi8(_mm_cmpeq_epi8(a_simd, b_simd)) == 0xFFFF
        }
    }
    // Fall back to scalar
    else {
        a == b
    }
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

    // NEON: 16 bytes at a time
    if len >= 16 {
        unsafe {
            let a_simd = vld1q_u8(a.as_ptr());
            let b_simd = vld1q_u8(b.as_ptr());
            vminvq_u8(vceqq_u8(a_simd, b_simd)) == 255
        }
    } else {
        a == b
    }
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
