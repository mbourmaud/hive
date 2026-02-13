use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Module-level PKCE store, lazily initialized.
pub fn pkce_store() -> &'static Arc<Mutex<HashMap<String, String>>> {
    use std::sync::OnceLock;
    static STORE: OnceLock<Arc<Mutex<HashMap<String, String>>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}
