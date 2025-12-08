use std::future::Future;
use std::pin::Pin;
use std::thread::JoinHandle;

/// Type alias for async task results
pub type AsyncTaskResult<T> = Result<T, String>;

/// Type alias for boxed async tasks
pub type AsyncTask<T> = Pin<Box<dyn Future<Output = AsyncTaskResult<T>> + Send + 'static>>;

/// Spawns a background thread that runs a Tokio async task
///
/// This is a convenience wrapper that:
/// 1. Spawns a new thread
/// 2. Creates a Tokio runtime (with error handling)
/// 3. Runs the async task in that runtime
/// 4. Returns a JoinHandle for tracking the thread
///
/// # Example
/// ```
/// let handle = spawn_api_task(move || {
///     Box::pin(async move {
///         api::likes::like_track(&token, track_id).await
///             .map_err(|e| e.to_string())
///     })
/// });
/// ```
pub fn spawn_api_task<F, T>(task_factory: F) -> JoinHandle<AsyncTaskResult<T>>
where
    F: FnOnce() -> AsyncTask<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match crate::utils::error_handling::create_runtime() {
            Ok(r) => r,
            Err(e) => {
                log::error!("[AsyncHelper] Failed to create runtime: {}", e);
                return Err(e);
            }
        };

        rt.block_on(task_factory())
    })
}

/// Spawns a background thread that runs an async task and sends the result via a channel
///
/// This variant is useful when you want to receive results via a channel instead of
/// joining the thread. The thread will exit after sending the result.
///
/// # Example
/// ```
/// let (tx, rx) = std::sync::mpsc::channel();
/// spawn_and_send(
///     move || Box::pin(async move {
///         api::fetch_data(&token).await.map_err(|e| e.to_string())
///     }),
///     tx
/// );
/// // Later: let result = rx.recv();
/// ```
pub fn spawn_and_send<F, T>(
    task_factory: F,
    tx: std::sync::mpsc::Sender<AsyncTaskResult<T>>,
) -> JoinHandle<()>
where
    F: FnOnce() -> AsyncTask<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match crate::utils::error_handling::create_runtime() {
            Ok(r) => r,
            Err(e) => {
                log::error!("[AsyncHelper] Failed to create runtime: {}", e);
                let _ = tx.send(Err(e));
                return;
            }
        };

        let result = rt.block_on(task_factory());
        let _ = tx.send(result);
    })
}

/// Fire-and-forget spawn for tasks where you don't need the result
///
/// Use this when you just want to run an async task in the background
/// and don't care about tracking it or getting the result.
///
/// # Example
/// ```
/// spawn_fire_and_forget(move || {
///     Box::pin(async move {
///         api::log_event(&token, event_data).await
///             .map_err(|e| e.to_string())
///     })
/// });
/// ```
pub fn spawn_fire_and_forget<F, T>(task_factory: F)
where
    F: FnOnce() -> AsyncTask<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match crate::utils::error_handling::create_runtime() {
            Ok(r) => r,
            Err(e) => {
                log::error!("[AsyncHelper] Failed to create runtime: {}", e);
                return;
            }
        };

        let _ = rt.block_on(task_factory());
    });
}
