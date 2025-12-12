use std::sync::{Mutex, MutexGuard};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::runtime::Runtime;

/// Creates a lightweight single-threaded Tokio runtime
///
/// Uses current_thread scheduler to avoid thread explosion (default multi-threaded
/// runtime spawns N worker threads where N = CPU cores). Multiple runtimes across
/// the app would create excessive threads (20+ on 4-core CPU).
///
/// Returns `Ok(Runtime)` if successful, or `Err(String)` with error message
pub fn create_runtime() -> Result<Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))
}

/// Safely locks a mutex with poisoning recovery
///
/// If the mutex is poisoned (previous holder panicked), this function
/// will recover by extracting the inner value. This is safe for our use case
/// where the data consistency is not critical (UI state, audio metrics).
///
/// Returns `Some(MutexGuard)` if successful, or `None` if lock failed
pub fn safe_lock<'a, T>(mutex: &'a Mutex<T>, context: &str) -> Option<MutexGuard<'a, T>> {
    match mutex.lock() {
        Ok(guard) => Some(guard),
        Err(poisoned) => {
            log::warn!("[{}] Mutex poisoned, recovering from panic", context);
            Some(poisoned.into_inner())
        }
    }
}

/// Safely locks a mutex, logging error if failed
///
/// Unlike `safe_lock`, this function does NOT recover from poisoning.
/// Use this for critical data where consistency matters.
///
/// Returns `Some(MutexGuard)` if successful, `None` if lock failed
#[allow(dead_code)]
pub fn safe_lock_or_log<'a, T>(mutex: &'a Mutex<T>, context: &str) -> Option<MutexGuard<'a, T>> {
    match mutex.lock() {
        Ok(guard) => Some(guard),
        Err(e) => {
            log::error!("[{}] Mutex lock failed: {}", context, e);
            None
        }
    }
}

// ============================================================================
// ATOMIC F32 HELPERS - Lock-free float storage using bit-casting
// ============================================================================

/// Store an f32 value in an AtomicU32 using lock-free operations
///
/// This uses bit-casting to store the float as a u32, allowing lock-free
/// atomic operations. Perfect for audio metrics that are read/written frequently.
///
/// # Example
/// ```
/// use std::sync::atomic::AtomicU32;
/// let atomic = AtomicU32::new(0);
/// store_f32_atomic(&atomic, 0.75);
/// ```
pub fn store_f32_atomic(atomic: &AtomicU32, value: f32) {
    atomic.store(value.to_bits(), Ordering::Relaxed);
}

/// Load an f32 value from an AtomicU32 using lock-free operations
///
/// This uses bit-casting to read the u32 as a float. Safe because
/// any bit pattern is a valid f32 (including NaN/Inf).
///
/// # Example
/// ```
/// use std::sync::atomic::AtomicU32;
/// let atomic = AtomicU32::new(0);
/// let value = load_f32_atomic(&atomic);
/// ```
pub fn load_f32_atomic(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}
