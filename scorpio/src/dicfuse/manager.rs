use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::OnceCell;

use dashmap::DashMap;

use super::Dicfuse;
use crate::util::config;

/// Global Dicfuse instance manager.
///
/// Provides a singleton Dicfuse instance that can be shared across multiple
/// Antares instances. This avoids redundant directory tree loading and
/// reduces memory usage in high-concurrency build scenarios.
///
/// # Thread Safety
///
/// The global instance is initialized only once using `OnceCell`, which
/// guarantees thread-safe lazy initialization. All subsequent calls to
/// `global()` will return a clone of the same `Arc<Dicfuse>` instance.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use scorpio::dicfuse::DicfuseManager;
///
/// #[tokio::main]
/// async fn main() {
///     // Get the global shared instance
///     let dicfuse = DicfuseManager::global().await;
///
///     // Multiple calls return the same instance
///     let dicfuse2 = DicfuseManager::global().await;
///     assert!(Arc::ptr_eq(&dicfuse, &dicfuse2));
/// }
/// ```
pub struct DicfuseManager;

static GLOBAL_DICFUSE: OnceCell<Arc<Dicfuse>> = OnceCell::const_new();
static DICFUSE_CACHE: OnceCell<DashMap<DicfuseCacheKey, Arc<OnceCell<Arc<Dicfuse>>>>> =
    OnceCell::const_new();

#[derive(Debug, Clone, Eq)]
struct DicfuseCacheKey {
    store_root: String,
    base_path: String,
}

impl PartialEq for DicfuseCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.store_root == other.store_root && self.base_path == other.base_path
    }
}

impl Hash for DicfuseCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.store_root.hash(state);
        self.base_path.hash(state);
    }
}

fn normalize_base_path(base_path: &str) -> String {
    if base_path.is_empty() || base_path == "/" {
        "/".to_string()
    } else {
        base_path.trim_end_matches('/').to_string()
    }
}

impl DicfuseManager {
    /// Get or initialize the global Dicfuse instance.
    ///
    /// This method is safe to call concurrently from multiple threads/tasks.
    /// The Dicfuse instance is initialized only once and then reused for all
    /// subsequent calls. This ensures that all Antares instances share the
    /// same read-only directory tree, avoiding redundant network requests
    /// and memory usage.
    ///
    /// # Returns
    ///
    /// An `Arc<Dicfuse>` pointing to the global shared instance.
    ///
    /// # Panics
    ///
    /// This method will panic if the Dicfuse initialization fails. In practice,
    /// this should only happen if there are critical system errors (e.g.,
    /// database initialization failures).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use scorpio::dicfuse::DicfuseManager;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let dicfuse = DicfuseManager::global().await;
    ///     // Use dicfuse...
    /// }
    /// ```
    pub async fn global() -> Arc<Dicfuse> {
        GLOBAL_DICFUSE
            .get_or_init(|| async {
                let dicfuse = Arc::new(Dicfuse::new().await);
                // Trigger import_arc immediately so directory tree starts loading.
                // Guarded so we don't start multiple concurrent imports for the same store.
                if dicfuse.store.try_start_import() {
                    let store_clone = dicfuse.store.clone();
                    tokio::spawn(async move {
                        super::store::import_arc(store_clone).await;
                    });
                }
                dicfuse
            })
            .await
            .clone()
    }

    /// Get or initialize a shared Dicfuse instance for a specific base path.
    ///
    /// This enables multiple Antares mounts that use the same `base_path` to share one Dicfuse
    /// instance (and thus one set of in-memory caches), improving stability and performance in
    /// high-concurrency build scenarios.
    ///
    /// The `store_root` is the configured `store_path` directory from config; per-base_path stores
    /// are isolated under that root to avoid sled DB lock conflicts.
    ///
    /// # TODO(dicfuse-antares-integration)
    /// - Add LRU eviction for cached Dicfuse instances to limit memory usage
    /// - Support instance prewarming for known build paths
    /// - Add health check / refresh mechanism for long-lived instances
    pub async fn for_base_path(base_path: &str) -> Arc<Dicfuse> {
        let store_root = config::store_path().to_string();
        Self::for_base_path_with_store_root(base_path, &store_root).await
    }

    /// Same as `for_base_path`, but allows explicitly specifying the store root directory.
    /// Useful for tests that want isolated on-disk state.
    pub async fn for_base_path_with_store_root(base_path: &str, store_root: &str) -> Arc<Dicfuse> {
        let normalized = normalize_base_path(base_path);

        // For the root view, prefer the global singleton when using the default store_root.
        if normalized == "/" && store_root == config::store_path() {
            return Self::global().await;
        }

        let cache = DICFUSE_CACHE.get_or_init(|| async { DashMap::new() }).await;

        let key = DicfuseCacheKey {
            store_root: store_root.to_string(),
            base_path: normalized.clone(),
        };

        let cell = cache
            .entry(key)
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone();

        cell.get_or_init(|| async move {
            // Use a deterministic per-base_path directory so multiple mounts can share it.
            // Keep it stable across process restarts for cache reuse.
            let store_path =
                super::compute_store_dir_for_base_path_with_store_root(store_root, &normalized);
            let _ = std::fs::create_dir_all(&store_path);

            let dicfuse = Arc::new(
                Dicfuse::new_with_base_path_and_store_path(&normalized, &store_path).await,
            );

            // IMPORTANT: Trigger import_arc immediately so the directory tree starts loading.
            // This is necessary because `import_arc` is normally called in `Filesystem::init()`
            // when FUSE mounts, but callers may need to wait_for_ready() BEFORE mounting
            // (e.g., the Antares daemon needs the root inode to be set up first).
            if dicfuse.store.try_start_import() {
                let store_clone = dicfuse.store.clone();
                tokio::spawn(async move {
                    super::store::import_arc(store_clone).await;
                });
            }

            dicfuse
        })
        .await
        .clone()
    }

    /// Create a new Dicfuse instance (for testing or special cases).
    ///
    /// This method creates a new, isolated Dicfuse instance that is not
    /// shared with other parts of the application. This is primarily
    /// useful for:
    ///
    /// - Unit tests that need isolated state
    /// - Special scenarios where you need a separate Dicfuse instance
    ///
    /// For normal use cases, prefer `global()` to benefit from shared
    /// state and reduced resource usage.
    ///
    /// # Returns
    ///
    /// A new `Arc<Dicfuse>` instance that is independent of the global instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use scorpio::dicfuse::DicfuseManager;
    ///
    /// #[tokio::test]
    /// async fn test_with_isolated_dicfuse() {
    ///     // Create an isolated instance for testing
    ///     let dicfuse: Arc<_> = DicfuseManager::new().await;
    ///     // Test with isolated state...
    /// }
    /// ```
    ///
    /// TODO(dicfuse-global-singleton): consider exposing a `new_with_store_path` async
    /// constructor to support multiple independent stores in tests instead of reusing
    /// the global on-disk database path configured in `scorpio_test.toml`.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() -> Arc<Dicfuse> {
        // IMPORTANT: avoid opening the global on-disk DB path (can conflict with the global singleton).
        // Use a unique temporary store root for isolated instances.
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp_root = format!("/tmp/scorpio_dicfuse_isolated_{}", nanos);
        let _ = std::fs::create_dir_all(&tmp_root);
        Arc::new(Dicfuse::new_with_store_path(&tmp_root).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial] // Serialize test execution to avoid database lock conflicts
    #[ignore = "Requires exclusive access to sled DB path; may fail locally if another scorpio/dicfuse process holds the lock"]
    async fn test_global_returns_same_instance() {
        let dic1 = DicfuseManager::global().await;
        let dic2 = DicfuseManager::global().await;

        // Both should point to the same allocation
        assert!(Arc::ptr_eq(&dic1, &dic2));
    }

    #[tokio::test]
    #[serial] // Serialize test execution to avoid database lock conflicts
    #[ignore = "Requires exclusive access to sled DB path; may fail locally if another scorpio/dicfuse process holds the lock"]
    async fn test_new_returns_different_instance() {
        // Database lock conflict explanation:
        // - All Dicfuse instances use the same database file: /home/master1/megadir/store/path.db
        // - sled::open() acquires a file lock that persists while the database is open
        // - If the global instance was initialized by a previous test, it holds the lock
        // - DicfuseManager::new() tries to open the same database file, causing a lock conflict
        //
        // This is expected behavior: in production, only one global instance exists.
        // The test verifies that global() and new() are different code paths.

        // Try to get global instance (may have been initialized by previous test)
        let global = DicfuseManager::global().await;

        // Attempt to create a new instance directly in this async context.
        // If global instance holds the sled DB lock, this may panic; we treat that
        // as an acceptable outcome and only assert when creation succeeds.
        let isolated = DicfuseManager::new().await;

        // Successfully created new instance - verify it's different from global
        assert!(
            !Arc::ptr_eq(&global, &isolated),
            "new() should return a different instance than global()"
        );
    }

    #[tokio::test]
    #[serial] // Serialize test execution to avoid database lock conflicts
    #[ignore = "Requires exclusive access to sled DB path; may fail locally if another scorpio/dicfuse process holds the lock"]
    async fn test_concurrent_global_access() {
        use tokio::task;

        // Spawn multiple tasks that concurrently access global()
        let handles: Vec<_> = (0..10)
            .map(|_| task::spawn(async { DicfuseManager::global().await }))
            .collect();

        let results: Vec<_> = futures::future::join_all(handles).await;
        let instances: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();

        // All instances should be the same
        for i in 1..instances.len() {
            assert!(Arc::ptr_eq(&instances[0], &instances[i]));
        }
    }
}
