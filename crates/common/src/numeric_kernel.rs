//! # Numerical Kernel: Log-Space Multiplicative Update with Stability Controls
//!
//! This module implements a **numerically stable core kernel** for performing constrained
//! multiplicative updates, crucial for maintaining **reproducibility**, **bounded precision**,
//! and **numerical stability** in iterative algorithms.
//!
//! ---
//!
//! ## Goals & Stability
//!
//! The primary goal is to perform the multiplication of two factors, `a × b`, on an
//! existing `old_value`, while enforcing four key stability constraints:
//!
//! 1. **Log-Space Stability:**  
//!    Mitigates cumulative floating-point drift in repeated multiplications by operating
//!    in log-space: `ln(a) + ln(b) → exp()`.
//!
//! 2. **Range Clamping:**  
//!    Constrains both factors (`a` and `b`) to a safe dynamic range `[min_r, max_r]` before calculation.
//!
//! 3. **Precision Quantization:**  
//!    Enforces deterministic rounding by snapping the final result to a discrete resolution
//!    defined by `quantum`, improving reproducibility across runs.
//!
//! 4. **Epsilon Gating (Idempotence):**  
//!    Prevents committing negligible updates when the computed change from `old_value`
//!    is smaller than a tolerance threshold `eps`, ensuring stable convergence without jitter.
//!
//! ---
//!
//! ## Core Mechanics: The `log_mul_eps` Process
//!
//! The update follows a sequential pipeline:
//!
//! 1. **Input Clamping:** Apply `min_r` and `max_r` bounds to both `a` and `b`.
//! 2. **Log-Space Multiplication:** Compute  
//!    `new_value_raw = exp(ln(a_clamped) + ln(b_clamped))`.
//! 3. **Quantization:** Round `new_value_raw` to the nearest multiple of `quantum`.  
//!    `quantized_value = round(new_value_raw / quantum) * quantum`
//! 4. **Epsilon Gate:** If `|quantized_value - old_value| < eps`,  
//!    return `old_value` (gate closed, no update).
//! 5. **Commit:** Otherwise, return `quantized_value` (gate open, update applied).
//!
//! ---
//!
//! ## Typical Use Cases
//!
//! - **Iterative Algorithms:** Numerical kernels, stochastic models, or dynamic simulations.
//! - **Bounded Precision Systems:** Financial models, physics engines, or control loops
//!   requiring deterministic rounding and controlled drift.
//! - **Idempotent Update Loops:** Systems where micro-changes should not accumulate jitter
//!   across iterations.
//!
//! ---
//!
//! ## Function
//! - [`log_mul_eps`]: Core function performing the log-space multiply–quantize–gate operation.

use std::f64;

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
        // Product is 1.000230013
        let precise_a = 1.0001;
        let precise_b = 1.00013;
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
