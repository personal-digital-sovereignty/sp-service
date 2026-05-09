//! ============================================================
//! sp-service — TurboQuant Tests
//! Tests for 4-bit vector quantization, Lloyd-Max codebook, packing
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::turboquant::{
        gram_schmidt_qr, lloyd_max_codebook, compute_midpoints,
        quantize_single, dequantize_single, pack_4bit, unpack_4bit,
        TurboState,
    };
    use ndarray::Array2;

    // ─────────────────────────────────────────────────────────
    // pack_4bit / unpack_4bit Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_pack_4bit_basic() {
        let indices = vec![0x0A, 0x0F, 0x03, 0x07];
        let packed = pack_4bit(&indices);
        assert_eq!(packed.len(), 2);
        // First byte: 0x0A << 4 | 0x0F = 0xAF
        assert_eq!(packed[0], 0xAF);
        // Second byte: 0x03 << 4 | 0x07 = 0x37
        assert_eq!(packed[1], 0x37);
    }

    #[test]
    fn test_pack_4bit_odd_length() {
        let indices = vec![0x05];
        let packed = pack_4bit(&indices);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0x50);
    }

    #[test]
    fn test_pack_4bit_empty() {
        let packed = pack_4bit(&[]);
        assert!(packed.is_empty());
    }

    #[test]
    fn test_unpack_4bit_roundtrip() {
        let original = vec![0x01, 0x0F, 0x0A, 0x03];
        let packed = pack_4bit(&original);
        let unpacked = unpack_4bit(&packed, original.len());
        assert_eq!(unpacked, original);
    }

    #[test]
    fn test_unpack_4bit_truncates_to_dim() {
        let packed = vec![0xAB];
        let unpacked = unpack_4bit(&packed, 1);
        assert_eq!(unpacked.len(), 1);
        assert_eq!(unpacked[0], 0x0A);
    }

    #[test]
    fn test_unpack_4bit_max_value() {
        let packed = vec![0xFF];
        let unpacked = unpack_4bit(&packed, 2);
        assert_eq!(unpacked, vec![0x0F, 0x0F]);
    }

    // ─────────────────────────────────────────────────────────
    // compute_midpoints Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_compute_midpoints_basic() {
        let codebook = vec![0.0, 0.5, 1.0];
        let midpoints = compute_midpoints(&codebook);
        assert_eq!(midpoints.len(), 2);
        assert!((midpoints[0] - 0.25).abs() < 1e-6);
        assert!((midpoints[1] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_compute_midpoints_single() {
        let codebook = vec![0.0, 1.0];
        let midpoints = compute_midpoints(&codebook);
        assert_eq!(midpoints.len(), 1);
        assert!((midpoints[0] - 0.5).abs() < 1e-6);
    }

    // compute_midpoints requires at least 2 elements, so we skip edge case tests

    #[test]
    fn test_compute_midpoints_negative() {
        let codebook = vec![-1.0, 0.0, 1.0];
        let midpoints = compute_midpoints(&codebook);
        assert_eq!(midpoints.len(), 2);
        assert!((midpoints[0] - (-0.5)).abs() < 1e-6);
        assert!((midpoints[1] - 0.5).abs() < 1e-6);
    }

    // ─────────────────────────────────────────────────────────
    // lloyd_max_codebook Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_lloyd_max_codebook_2bit() {
        let codebook = lloyd_max_codebook(768, 2);
        assert_eq!(codebook.len(), 4); // 2^2 = 4 centroids
        // Check sorted
        for i in 1..codebook.len() {
            assert!(codebook[i] >= codebook[i - 1]);
        }
    }

    #[test]
    fn test_lloyd_max_codebook_4bit() {
        let codebook = lloyd_max_codebook(768, 4);
        assert_eq!(codebook.len(), 16); // 2^4 = 16 centroids
        for i in 1..codebook.len() {
            assert!(codebook[i] >= codebook[i - 1]);
        }
    }

    #[test]
    fn test_lloyd_max_codebook_1bit() {
        let codebook = lloyd_max_codebook(768, 1);
        assert_eq!(codebook.len(), 2); // 2^1 = 2 centroids
    }

    // ─────────────────────────────────────────────────────────
    // gram_schmidt_qr Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_gram_schmidt_qr_orthogonal() {
        let a = Array2::from(vec![
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
        ]);
        let q = gram_schmidt_qr(&a);

        // Check orthogonality: Q^T * Q ≈ I
        let qtq = q.t().dot(&q);
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((qtq[[i, j]] - expected).abs() < 1e-6,
                    "qtq[{},{}] = {} expected {}", i, j, qtq[[i, j]], expected);
            }
        }
    }

    #[test]
    fn test_gram_schmidt_qr_2x2() {
        let a = Array2::from(vec![
            [1.0, 0.0],
            [0.0, 1.0],
        ]);
        let q = gram_schmidt_qr(&a);
        // Identity should stay identity
        assert!((q[[0, 0]] - 1.0).abs() < 1e-6);
        assert!((q[[1, 1]] - 1.0).abs() < 1e-6);
    }

    // ─────────────────────────────────────────────────────────
    // TurboState Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_turbo_state_creation() {
        let state = TurboState::new(768, 4, 42);
        assert_eq!(state.rotation.dim(), (768, 768));
        assert_eq!(state.codebook.len(), 16); // 2^4
    }

    #[test]
    fn test_turbo_state_small_dim() {
        let state = TurboState::new(8, 2, 99);
        assert_eq!(state.rotation.dim(), (8, 8));
        assert_eq!(state.codebook.len(), 4); // 2^2
    }

    // ─────────────────────────────────────────────────────────
    // quantize_single / dequantize_single Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_quantize_dequantize_roundtrip() {
        let state = TurboState::new(64, 4, 42);
        // Create a test embedding
        let embedding: Vec<f32> = (0..64).map(|i| (i as f32) / 64.0).collect();

        let (packed, norm) = quantize_single(&embedding, &state);
        let reconstructed = dequantize_single(&packed, norm, 64, &state);

        assert_eq!(reconstructed.len(), 64);

        // Check cosine similarity is high (quantization introduces some error)
        let dot: f32 = embedding.iter().zip(reconstructed.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = reconstructed.iter().map(|x| x * x).sum::<f32>().sqrt();
        let cos_sim = dot / (norm_a * norm_b);

        assert!(cos_sim > 0.8, "Cosine similarity too low: {}", cos_sim);
    }

    #[test]
    fn test_quantize_zero_vector() {
        let state = TurboState::new(64, 4, 42);
        let embedding = vec![0.0; 64];

        let (packed, norm) = quantize_single(&embedding, &state);
        // Norm should be ~0 or handled gracefully
        assert!(norm < 0.001);
        // Should still produce valid packed output
        assert!(!packed.is_empty());
    }

    #[test]
    fn test_quantize_packed_size() {
        let state = TurboState::new(128, 4, 42);
        let embedding: Vec<f32> = (0..128).map(|i| (i as f32) / 128.0).collect();

        let (packed, _) = quantize_single(&embedding, &state);
        // 128 values → 64 bytes (2 per byte at 4-bit)
        assert_eq!(packed.len(), 64);
    }
}
