use ndarray::{Array1, Array2};
use ndarray_linalg::QR;
use rand::SeedableRng;
use rand_distr::{Distribution, StandardNormal};
use statrs::distribution::{Beta, Continuous};

use lazy_static::lazy_static;

lazy_static! {
    pub static ref TURBO_STATE: TurboState = {
        // Nomic Embed Text = 768 dims | 4-bits param (compression max)
        TurboState::new(768, 4, 42)
    };
}

/// Estado pre-computado do TurboQuant. Contém a matriz de Rotação (Haar Measure)
/// e o respectivo codebook de centróides calculados via Lloyd-Max.
pub struct TurboState {
    pub rotation: Array2<f32>,
    pub codebook: Vec<f32>,
}

impl TurboState {
    pub fn new(dim: usize, bits: u8, seed: u64) -> Self {
        println!("🧠 [TurboQuant] Computando matriz Haar (Orthogonal Rotation) para Dim={}...", dim);
        let rotation = fit_rotation(dim, seed);
        
        println!("🧠 [TurboQuant] Ajustando Centróides Escalares em {}-bits (Lloyd-Max Convergence)...", bits);
        let codebook = lloyd_max_codebook(dim, bits);
        
        Self { rotation, codebook }
    }
}

/// 1. Distribui a energia/variância dos embeddings através das dimensões via Matriz Ortogonal
pub fn fit_rotation(dim: usize, seed: u64) -> Array2<f32> {
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let g: Array2<f64> = Array2::from_shape_fn((dim, dim), |_| StandardNormal.sample(&mut rng));
    let (q, _) = g.qr().expect("Falha ao gerar decomposição QR para Matriz de Rotação");
    q.mapv(|x: f64| x as f32)
}

/// 2. Quantizador Escalar (Encontra os 2^B centróides para reconstrução de menor erro)
pub fn lloyd_max_codebook(dim: usize, bits: u8) -> Vec<f32> {
    let k = 1usize << bits;
    let num_grid = 10_000;
    
    // Beta((d-1)/2, (d-1)/2)
    let alpha = (dim as f64 - 1.0) / 2.0;
    let beta_dist = Beta::new(alpha, alpha).unwrap();

    let mut xs = Vec::with_capacity(num_grid);
    let mut pdf = Vec::with_capacity(num_grid);
    
    for i in 0..num_grid {
        let t = -1.0 + 1e-9 + (2.0 - 2e-9) * (i as f64) / (num_grid as f64 - 1.0);
        xs.push(t);
        let u = (t + 1.0) / 2.0;
        pdf.push(beta_dist.pdf(u) / 2.0);
    }
    
    // Inicialização simples de centróides distribuídos linearmente no espaço Beta
    let mut centroids: Vec<f64> = (0..k)
        .map(|i| -0.1 + 0.2 * (i as f64) / (k as f64 - 1.0)) 
        .collect();
        
    for _ in 0..100 {
        let mut denom = vec![0.0f64; k];
        let mut numer = vec![0.0f64; k];
        
        for i in 0..num_grid {
            let x = xs[i];
            let p = pdf[i];
            
            // Search Bucket
            let mut best_k = 0;
            let mut min_dist = f64::MAX;
            for (idx, &c) in centroids.iter().enumerate() {
                let dist = (x - c).abs();
                if dist < min_dist {
                    min_dist = dist;
                    best_k = idx;
                }
            }
            
            numer[best_k] += p * x;
            denom[best_k] += p;
        }
        
        for i in 0..k {
            if denom[i] > 1e-12 {
                centroids[i] = numer[i] / denom[i];
            }
        }
    }
    
    centroids.sort_by(|a, b| a.partial_cmp(b).unwrap());
    centroids.iter().map(|&x| x as f32).collect()
}

/// 3. Computa fronteiras (midpoints) entre os centróides para o searchsorted da Quantização.
pub fn compute_midpoints(codebook: &[f32]) -> Vec<f32> {
    let mut midpoints = Vec::with_capacity(codebook.len() - 1);
    for window in codebook.windows(2) {
        midpoints.push((window[0] + window[1]) / 2.0);
    }
    midpoints
}

/// 4. Quantiza e comprime um único vetor, retornando o byte array packado e a norma escalar (f32)
pub fn quantize_single(embedding: &[f32], state: &TurboState) -> (Vec<u8>, f32) {
    let dim = embedding.len();
    
    // A) Normaliza na Esfera Unitária
    let norm = (embedding.iter().map(|&x| (x as f64).powi(2)).sum::<f64>()).sqrt() as f32;
    let norm_safe = if norm == 0.0 { 1.0 } else { norm };
    
    // B) Rotaciona (Espalha a Variância)
    let emb_arr = Array1::from_shape_vec(dim, embedding.iter().map(|&x| x / norm_safe).collect()).unwrap();
    let rotated = emb_arr.dot(&state.rotation.t());
    
    // C) Search-Sorted nos Centróides (Lloyd-Max Codebook)
    let midpoints = compute_midpoints(&state.codebook);
    let mut indices = Vec::with_capacity(dim);
    for &val in rotated.iter() {
        let idx = midpoints.partition_point(|&m| m < val) as u8;
        indices.push(idx);
    }
    
    // D) Packing (4-bit -> 2 índices por Byte)
    let packed = pack_4bit(&indices);
    
    (packed, norm)
}

/// 5. Dequantiza o byte pack para comparação por cosseno no espaço real
pub fn dequantize_single(packed: &[u8], norm: f32, dim: usize, state: &TurboState) -> Vec<f32> {
    // Unpack (2 índices por Byte -> 1 índice por Byte)
    let indices = unpack_4bit(packed, dim);
    
    // Recria os valores escalares baseado nos centróides
    let y_hat: Vec<f32> = indices.into_iter().map(|i| state.codebook[i as usize]).collect();
    let y_hat_arr = Array1::from_shape_vec(dim, y_hat).unwrap();
    
    // Reverse Rotation
    let x_hat = y_hat_arr.dot(&state.rotation);
    
    // Reverse Normalization
    x_hat.into_raw_vec_and_offset().0.into_iter().map(|v| v * norm).collect()
}

// Utilitários de Bit-Packing
fn pack_4bit(indices: &[u8]) -> Vec<u8> {
    indices.chunks(2)
        .map(|pair| {
            let high = pair[0] << 4;
            let low = pair.get(1).copied().unwrap_or(0) & 0x0F;
            high | low
        })
        .collect()
}

fn unpack_4bit(packed: &[u8], dim: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(dim);
    for &byte in packed {
        out.push(byte >> 4);
        out.push(byte & 0x0F);
    }
    out.truncate(dim);
    out
}
