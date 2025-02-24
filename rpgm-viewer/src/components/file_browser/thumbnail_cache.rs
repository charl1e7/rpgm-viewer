use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct ThumbnailCache {
    cache: HashMap<PathBuf, (egui::TextureHandle, Instant)>,
    max_size: usize,
    ttl: Duration,
    pending_loads: HashSet<PathBuf>,
    counter: usize,
}

impl ThumbnailCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
            ttl,
            ..Default::default()
        }
    }

    pub fn get(&mut self, path: &Path) -> Option<egui::TextureHandle> {
        self.counter += 1;
        if self.counter % 100 == 0 {
            self.cleanup_expired();
        }

        if let Some((texture, timestamp)) = self.cache.get(path) {
            if timestamp.elapsed() < self.ttl {
                return Some(texture.clone());
            }
            self.cache.remove(path);
        }
        None
    }

    fn cleanup_expired(&mut self) {
        let _now = Instant::now();
        self.cache
            .retain(|_, (_, timestamp)| timestamp.elapsed() < self.ttl);
    }

    pub fn is_pending(&self, path: &Path) -> bool {
        self.pending_loads.contains(path)
    }

    pub fn mark_pending(&mut self, path: PathBuf) {
        self.pending_loads.insert(path);
    }

    pub fn unmark_pending(&mut self, path: &Path) {
        self.pending_loads.remove(path);
    }

    pub fn insert(&mut self, path: PathBuf, texture: egui::TextureHandle) {
        while self.cache.len() >= self.max_size {
            if let Some((oldest_key, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, timestamp))| timestamp.elapsed())
            {
                let oldest_key = oldest_key.clone();
                self.cache.remove(&oldest_key);
            } else {
                break;
            }
        }

        self.cache.insert(path, (texture, Instant::now()));
    }

    pub fn remove(&mut self, path: &Path) {
        self.cache.remove(path);
        self.pending_loads.remove(path);
    }
}
