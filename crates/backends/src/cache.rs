use async_trait::async_trait;
use lazypackage_core::domain::Package;
use lazypackage_core::traits::{PackageSource, Result};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct Cached<S: PackageSource> {
    inner: S,
    ttl: Duration,
    installed_cache: RwLock<Option<(Instant, Vec<Package>)>>,
    search_cache: RwLock<std::collections::HashMap<String, (Instant, Vec<Package>)>>,
}

impl<S: PackageSource> Cached<S> {
    pub fn new(inner: S, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            installed_cache: RwLock::new(None),
            search_cache: RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl<S: PackageSource> PackageSource for Cached<S> {
    async fn list_installed(&self) -> Result<Vec<Package>> {
        {
            let cache = self.installed_cache.read().await;
            if let Some((time, pkgs)) = &*cache {
                if time.elapsed() < self.ttl {
                    return Ok(pkgs.clone());
                }
            }
        }

        let pkgs = self.inner.list_installed().await?;

        let mut cache = self.installed_cache.write().await;
        *cache = Some((Instant::now(), pkgs.clone()));

        Ok(pkgs)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        {
            let cache = self.search_cache.read().await;
            if let Some((time, pkgs)) = cache.get(query) {
                if time.elapsed() < self.ttl {
                    return Ok(pkgs.clone());
                }
            }
        }

        let pkgs = self.inner.search(query).await?;

        let mut cache = self.search_cache.write().await;
        cache.insert(query.to_string(), (Instant::now(), pkgs.clone()));

        Ok(pkgs)
    }

    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering {
        self.inner.compare_versions(a, b)
    }
}
