use std::sync::{Arc, OnceLock, RwLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const DIRECT_FETCH_MAX_CONCURRENT_DEFAULT: usize = 200;
const DIRECT_FETCH_MAX_CONCURRENT_MIN: usize = 1;
const DIRECT_FETCH_MAX_CONCURRENT_MAX: usize = 1000;

static DIRECT_FETCH_SEMAPHORE: OnceLock<RwLock<Arc<Semaphore>>> = OnceLock::new();

fn direct_fetch_semaphore_store() -> &'static RwLock<Arc<Semaphore>> {
    DIRECT_FETCH_SEMAPHORE.get_or_init(|| {
        RwLock::new(Arc::new(Semaphore::new(
            DIRECT_FETCH_MAX_CONCURRENT_DEFAULT,
        )))
    })
}

pub fn sanitize_direct_fetch_max_concurrent(value: u64) -> u64 {
    value.clamp(
        DIRECT_FETCH_MAX_CONCURRENT_MIN as u64,
        DIRECT_FETCH_MAX_CONCURRENT_MAX as u64,
    )
}

pub fn default_direct_fetch_max_concurrent() -> u64 {
    DIRECT_FETCH_MAX_CONCURRENT_DEFAULT as u64
}

pub fn configure_direct_fetch_max_concurrent(limit: u64) {
    let limit = sanitize_direct_fetch_max_concurrent(limit) as usize;
    let mut guard = direct_fetch_semaphore_store()
        .write()
        .expect("direct fetch semaphore poisoned");
    *guard = Arc::new(Semaphore::new(limit));
}

pub async fn acquire_direct_fetch_permit() -> OwnedSemaphorePermit {
    let semaphore = direct_fetch_semaphore_store()
        .read()
        .expect("direct fetch semaphore poisoned")
        .clone();
    semaphore
        .acquire_owned()
        .await
        .expect("direct fetch semaphore closed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reconfigures_semaphore_limit() {
        configure_direct_fetch_max_concurrent(4);
        let semaphore = direct_fetch_semaphore_store()
            .read()
            .expect("direct fetch semaphore poisoned")
            .clone();
        assert_eq!(semaphore.available_permits(), 4);

        let _p1 = semaphore.acquire().await.expect("permit");
        let _p2 = semaphore.acquire().await.expect("permit");
        let _p3 = semaphore.acquire().await.expect("permit");
        let _p4 = semaphore.acquire().await.expect("permit");
        assert_eq!(semaphore.available_permits(), 0);

        configure_direct_fetch_max_concurrent(8);
        let refreshed = direct_fetch_semaphore_store()
            .read()
            .expect("direct fetch semaphore poisoned")
            .clone();
        assert_eq!(refreshed.available_permits(), 8);
    }
}
