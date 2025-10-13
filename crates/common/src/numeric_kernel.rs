use std::f64;

/// [Task 3] Implements precision clamping, log-space multiplication, and the epsilon gate.
pub fn log_mul_eps(
    old_value: f64,
    a: f64,
    b: f64,
    eps: f64,
    min_r: f64,
    max_r: f64,
    quantum: f64,
) -> f64 {
    let a_clamped = a.clamp(min_r, max_r);
    let b_clamped = b.clamp(min_r, max_r);

    // Multiplication via log-space addition (ln(a) + ln(b)) mitigates cumulative floating-point errors.
    let log_product = a_clamped.ln() + b_clamped.ln();
    let new_value_raw = log_product.exp();

    let quantized_value = (new_value_raw / quantum).round() * quantum;

    if (quantized_value - old_value).abs() < eps {
        return old_value;
    }

    quantized_value
}

#[cfg(test)]
mod numerical_kernel_tests {
    use super::*;

    const QUANTUM: f64 = 0.0001;
    const MIN_R: f64 = 0.5;
    const MAX_R: f64 = 2.0;

    // Helper to check for approximate equality (due to expected quantization/f64 math)
    fn assert_approx_eq(a: f64, b: f64) {
        assert!(
            (a - b).abs() < QUANTUM / 2.0,
            "{} is not approximately equal to {}",
            a,
            b
        );
    }

    /// Test case for near-1.0 values.
    #[test]
    fn test_near_one_value_precision() {
        // Expected value: 1.0001 * 1.0001 = 1.00020001
        // Quantized to 0.0001: 1.0002
        let result = log_mul_eps(1.0, 1.0001, 1.0001, 1e-12, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, 1.0002);
    }

    /// Test standard multiplication and quantization.
    #[test]
    fn test_quantization() {
        // Expected product: 1.5 * 1.3 = 1.95.
        // With Q=0.0001, no change.
        let result = log_mul_eps(1.0, 1.5, 1.3, 1e-12, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, 1.9500);

        // Test value that must be rounded: 1.00023 -> 1.0002
        let precise_a = 1.0001;
        let precise_b = 1.00013; // Product is 1.000230013
        let result = log_mul_eps(1.0, precise_a, precise_b, 1e-12, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, 1.0002);
    }

    #[test]
    fn test_clamps() {
        // Input 'a' is too high (3.0 > 2.0). Should be clamped to 2.0.
        // Product: 2.0 * 1.0 = 2.0
        let result = log_mul_eps(1.0, 3.0, 1.0, 1e-12, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, 2.0);

        // Input 'b' is too low (0.1 < 0.5). Should be clamped to 0.5.
        // Product: 1.0 * 0.5 = 0.5
        let result = log_mul_eps(1.0, 1.0, 0.1, 1e-12, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, 0.5);
    }

    /// Test tiny eps (no gate effect).
    #[test]
    fn test_tiny_epsilon() {
        let old = 1.0;
        let new_calc = 1.0002;
        let tiny_eps = 1e-15;

        // Difference is 0.0002. Since 0.0002 > tiny_eps, the gate opens.
        let result = log_mul_eps(old, 1.0, 1.0002, tiny_eps, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, new_calc);
    }

    /// Test the epsilon gate preventing minor updates.
    #[test]
    fn test_epsilon_gate_closed() {
        let old = 1.0;
        let large_eps = 0.1;

        // Calculated new_value (quantized) is 1.0002.
        // Difference is 0.0002. Since 0.0002 < 0.1, the gate should return old.
        let result = log_mul_eps(old, 1.0, 1.0002, large_eps, MIN_R, MAX_R, QUANTUM);
        assert_approx_eq(result, old); // Assert old_value is returned
    }

    /// Test idempotence.
    #[test]
    fn test_idempotence() {
        let initial_old = 1.0;
        let large_eps = 1e-6;

        let new_committed_value = log_mul_eps(
            initial_old,
            1.0001,
            1.0001,
            large_eps,
            MIN_R,
            MAX_R,
            QUANTUM,
        );
        assert_approx_eq(new_committed_value, 1.0002);

        // Run the function again, using the committed value as the new 'old_value'.
        // The new calculation will produce a raw value infinitesimally close to new_committed_value (1.0002).
        // Since |new_calc - new_committed_value| < large_eps, the gate must close.
        let final_stable_value = log_mul_eps(
            new_committed_value,
            1.0001,
            1.0001,
            large_eps,
            MIN_R,
            MAX_R,
            QUANTUM,
        );

        assert_approx_eq(final_stable_value, new_committed_value);
    }
}
