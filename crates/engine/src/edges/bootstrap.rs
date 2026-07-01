use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::edges::types::BootstrapResult;

pub struct BootstrapConfig {
    pub iterations: usize,
    pub block_size: usize,
    pub seed: Option<u64>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            iterations: 10000,
            block_size: 5,
            seed: Some(42),
        }
    }
}

pub fn run_bootstrap(returns: &[f64], cfg: &BootstrapConfig) -> BootstrapResult {
    if returns.is_empty() || returns.len() < 2 {
        return BootstrapResult {
            p_value: 1.0,
            is_significant: false,
            mean_return: 0.0,
            confidence_95_lower: 0.0,
            confidence_95_upper: 0.0,
            iterations: 0,
        };
    }

    let n = returns.len();

    let observed_mean = returns.iter().sum::<f64>() / n as f64;

    let mean_centered: Vec<f64> = returns.iter().map(|r| r - observed_mean).collect();

    let mut rng = if let Some(seed) = cfg.seed {
        StdRng::seed_from_u64(seed)
    } else {
        StdRng::from_entropy()
    };

    let mut bootstrap_means: Vec<f64> = Vec::with_capacity(cfg.iterations);

    let block_size = cfg.block_size.min(n);

    for _ in 0..cfg.iterations {
        let mut sample = Vec::with_capacity(n);
        while sample.len() < n {
            let start = (rng.gen::<f64>() * (n - block_size) as f64) as usize;
            let end = (start + block_size).min(n);
            sample.extend_from_slice(&mean_centered[start..end]);
        }
        sample.truncate(n);

        let boot_mean = sample.iter().sum::<f64>() / n as f64;
        bootstrap_means.push(boot_mean);
    }

    bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let lower_idx = (0.025 * cfg.iterations as f64) as usize;
    let upper_idx = (0.975 * cfg.iterations as f64) as usize;

    let ci_lower = bootstrap_means[lower_idx.min(bootstrap_means.len() - 1)];
    let ci_upper = bootstrap_means[upper_idx.min(bootstrap_means.len() - 1)];

    let null_hypothesis_mean = 0.0;

    let obs_diff = (observed_mean - null_hypothesis_mean).abs();

    let count_more_extreme = bootstrap_means
        .iter()
        .filter(|&&m| m.abs() >= obs_diff)
        .count();

    let p_value = count_more_extreme as f64 / cfg.iterations as f64;

    BootstrapResult {
        p_value,
        is_significant: p_value < 0.05,
        mean_return: observed_mean,
        confidence_95_lower: ci_lower,
        confidence_95_upper: ci_upper,
        iterations: cfg.iterations,
    }
}

fn compute_skewness(returns: &[f64]) -> f64 {
    if returns.len() < 3 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let m3 = returns.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / n;
    let m2 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    if m2 < 1e-12 {
        return 0.0;
    }
    m3 / m2.powf(1.5)
}

pub fn compute_return_skewness(returns: &[f64]) -> f64 {
    compute_skewness(returns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_empty() {
        let result = run_bootstrap(&[], &BootstrapConfig::default());
        assert_eq!(result.iterations, 0);
        assert!(!result.is_significant);
    }

    #[test]
    fn test_bootstrap_single_value() {
        let result = run_bootstrap(&[1.0], &BootstrapConfig::default());
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn test_bootstrap_random_noise_not_significant() {
        let noise: Vec<f64> = (0..1000)
            .map(|i| (i as f64 * 0.1).sin() * 0.01)
            .collect();
        let result = run_bootstrap(
            &noise,
            &BootstrapConfig {
                iterations: 2000,
                block_size: 5,
                seed: Some(42),
            },
        );
        assert!(result.p_value > 0.05 || !result.is_significant);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_bootstrap_strong_signal_significant() {
        let signal: Vec<f64> = (0..1000)
            .map(|_| 0.3)
            .collect();
        let result = run_bootstrap(
            &signal,
            &BootstrapConfig {
                iterations: 2000,
                block_size: 10,
                seed: Some(42),
            },
        );
        assert!(result.p_value < 0.05);
        assert!(result.is_significant);
    }

    #[test]
    fn test_skewness_positive() {
        let returns = vec![0.5, 0.5, 0.5, 0.5, 0.5, 5.0];
        let skew = compute_skewness(&returns);
        assert!(skew > 0.0, "Expected positive skewness, got {}", skew);
    }

    #[test]
    fn test_skewness_symmetric() {
        let returns = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let skew = compute_skewness(&returns);
        assert!(skew.abs() < 0.01);
    }
}
