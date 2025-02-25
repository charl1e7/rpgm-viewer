use log::{debug, error, info, trace};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub struct ThumbnailTask {
    pub path: PathBuf,
    pub decrypter: Arc<rpgm_enc::Decrypter>,
    pub compression_size: u32,
}

pub struct ThumbnailResult {
    pub path: PathBuf,
    pub texture_data: Option<(Vec<u8>, [usize; 2])>,
}

struct ThreadChannels {
    sender: mpsc::Sender<ThumbnailTask>,
    receiver: mpsc::Receiver<ThumbnailResult>,
}

#[derive(Default)]
pub struct ThumbnailCache {
    cache: HashMap<PathBuf, (egui::TextureHandle, Instant)>,
    max_size: usize,
    ttl: Duration,
    pending_loads: HashSet<PathBuf>,
    failed_loads: HashSet<PathBuf>,
    counter: usize,
    channels: Option<Arc<ThreadChannels>>,
    worker_running: bool,
}

impl ThumbnailCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        info!(
            "Creating new ThumbnailCache: max_size={}, ttl={:?}",
            max_size, ttl
        );

        let (task_tx, task_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        Self::start_worker_thread(task_rx, result_tx.clone());

        let channels = Arc::new(ThreadChannels {
            sender: task_tx,
            receiver: result_rx,
        });

        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
            ttl,
            pending_loads: HashSet::new(),
            failed_loads: HashSet::new(),
            counter: 0,
            channels: Some(channels),
            worker_running: true,
        }
    }

    fn start_worker_thread(
        task_rx: mpsc::Receiver<ThumbnailTask>,
        result_tx: mpsc::Sender<ThumbnailResult>,
    ) {
        info!("Starting background thread for thumbnail processing");
        thread::spawn(move || {
            debug!("Background thread started");
            let thread_id = thread::current().id();
            info!("Thumbnail thread started: {:?}", thread_id);

            while let Ok(task) = task_rx.recv() {
                debug!("Received thumbnail processing task: {:?}", task.path);
                let result = Self::process_thumbnail_task(task);
                debug!(
                    "Thumbnail processing completed: {:?}, success: {}",
                    result.path,
                    result.texture_data.is_some()
                );
                if result_tx.send(result).is_err() {
                    error!("Failed to send thumbnail processing result!");
                    break;
                }
            }
            debug!("Background thread terminated");
        });
    }

    fn process_thumbnail_task(task: ThumbnailTask) -> ThumbnailResult {
        let path = task.path.clone();
        trace!("Processing file: {:?}", path);

        let result = match std::fs::read(&task.path) {
            Ok(file_data) => {
                trace!("File successfully read: {} bytes", file_data.len());
                let mut rpg_file = match rpgm_enc::RPGFile::new(task.path.clone()) {
                    Ok(file) => file,
                    Err(e) => {
                        error!("Error creating RPGFile: {:?}, {:?}", path, e);
                        return ThumbnailResult {
                            path,
                            texture_data: None,
                        };
                    }
                };
                rpg_file.set_content(file_data);

                let image_data = if rpg_file.is_encrypted() {
                    trace!("File is encrypted, performing decryption");
                    match task.decrypter.decrypt(rpg_file.content().unwrap()) {
                        Ok(content) => {
                            trace!("Decryption successful: {} bytes", content.len());
                            content
                        }
                        Err(e) => {
                            error!("Error during decryption: {:?}, {:?}", path, e);
                            return ThumbnailResult {
                                path,
                                texture_data: None,
                            };
                        }
                    }
                } else {
                    trace!("File is not encrypted");
                    rpg_file.content().unwrap_or_default().to_vec()
                };

                match image::load_from_memory(&image_data) {
                    Ok(img) => {
                        let thumbnail = img.thumbnail(task.compression_size, task.compression_size);
                        let image_buffer = thumbnail.to_rgb8();
                        let dimensions = [thumbnail.width() as usize, thumbnail.height() as usize];
                        trace!("Thumbnail created: {}x{}", dimensions[0], dimensions[1]);
                        Some((image_buffer.as_raw().to_vec(), dimensions))
                    }
                    Err(e) => {
                        error!("Error loading image: {:?}, error: {:?}", path, e);
                        error!("File will be added to the problematic list and will not be requested again");
                        None
                    }
                }
            }
            Err(e) => {
                error!("Error reading file: {:?}, {:?}", path, e);
                None
            }
        };

        ThumbnailResult {
            path,
            texture_data: result,
        }
    }

    fn ensure_initialized(&mut self) {
        if self.channels.is_none() {
            info!("Initializing ThumbnailCache channels");

            let (task_tx, task_rx) = mpsc::channel();
            let (result_tx, result_rx) = mpsc::channel();

            Self::start_worker_thread(task_rx, result_tx.clone());

            let channels = Arc::new(ThreadChannels {
                sender: task_tx,
                receiver: result_rx,
            });

            self.channels = Some(channels);
            self.worker_running = true;
        }
    }

    pub fn request_thumbnail(
        &mut self,
        path: &Path,
        decrypter: &rpgm_enc::Decrypter,
        compression_size: u32,
    ) {
        if self.is_pending(path) {
            trace!("Thumbnail loading already pending: {:?}", path);
            return;
        }

        if self.failed_loads.contains(path) {
            trace!("Skipping previously failed thumbnail load: {:?}", path);
            return;
        }

        self.ensure_initialized();

        match &self.channels {
            Some(channels) => {
                debug!("Request to load thumbnail: {:?}", path);
                let sender = channels.sender.clone();
                self.mark_pending(path.to_path_buf());

                let decrypter_arc = Arc::new(decrypter.clone());

                let task = ThumbnailTask {
                    path: path.to_path_buf(),
                    decrypter: decrypter_arc,
                    compression_size,
                };

                if sender.send(task).is_err() {
                    error!("Error sending task to background thread: {:?}", path);
                    self.unmark_pending(path);
                } else {
                    debug!("Task successfully sent to background thread: {:?}", path);
                }
            }
            None => {
                error!("Sender for background thread not available!");
                self.ensure_initialized();
            }
        }
    }

    pub fn process_results(&mut self, ctx: &egui::Context) -> Vec<(PathBuf, egui::TextureHandle)> {
        let mut loaded_thumbnails = Vec::new();

        self.ensure_initialized();

        let mut results = Vec::new();
        match &self.channels {
            Some(channels) => {
                while let Ok(result) = channels.receiver.try_recv() {
                    debug!("Received thumbnail loading result: {:?}", result.path);
                    results.push(result);
                }
            }
            None => {
                error!("Receiver for background thread not available!");
                self.ensure_initialized();
                return loaded_thumbnails;
            }
        }

        for result in results {
            let path = result.path.clone();

            if let Some(texture_data) = result.texture_data {
                debug!("Loading texture for: {:?}", path);
                let (raw_data, dimensions) = texture_data;

                let texture = ctx.load_texture(
                    format!("thumb_{}", path.file_name().unwrap().to_string_lossy()),
                    egui::ColorImage::from_rgb([dimensions[0], dimensions[1]], &raw_data),
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Linear,
                        minification: egui::TextureFilter::Linear,
                        ..Default::default()
                    },
                );

                debug!("Texture created: {:?}", path);
                self.insert(path.clone(), texture.clone());
                loaded_thumbnails.push((path.clone(), texture));
            } else {
                debug!("Failed to get thumbnail data for: {:?}", path);
                self.failed_loads.insert(path.clone());
                info!("File marked as problematic and will be skipped: {:?}", path);
            }

            self.unmark_pending(&path);
        }

        if !loaded_thumbnails.is_empty() {
            info!("Loaded {} thumbnails", loaded_thumbnails.len());
        }

        loaded_thumbnails
    }

    pub fn get(&mut self, path: &Path) -> Option<egui::TextureHandle> {
        self.counter += 1;
        if self.counter % 100 == 0 {
            self.cleanup_expired();
        }

        if let Some((texture, timestamp)) = self.cache.get(path) {
            if timestamp.elapsed() < self.ttl {
                trace!("Retrieved thumbnail from cache: {:?}", path);
                return Some(texture.clone());
            }
            trace!("Thumbnail expired: {:?}", path);
            self.cache.remove(path);
        }
        None
    }

    fn cleanup_expired(&mut self) {
        let before_count = self.cache.len();
        self.cache
            .retain(|_, (_, timestamp)| timestamp.elapsed() < self.ttl);
        let after_count = self.cache.len();
        if before_count != after_count {
            debug!(
                "Cleaning expired thumbnails: removed {}",
                before_count - after_count
            );
        }
    }

    pub fn is_pending(&self, path: &Path) -> bool {
        self.pending_loads.contains(path)
    }

    pub fn is_failed(&self, path: &Path) -> bool {
        self.failed_loads.contains(path)
    }

    pub fn has_pending_loads(&self) -> bool {
        !self.pending_loads.is_empty()
    }

    pub fn mark_pending(&mut self, path: PathBuf) {
        trace!("Marking thumbnail as pending load: {:?}", path);
        self.pending_loads.insert(path);
    }

    pub fn unmark_pending(&mut self, path: &Path) {
        trace!("Removing pending load mark: {:?}", path);
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
                trace!("Removed old thumbnail from cache: {:?}", oldest_key);
            } else {
                break;
            }
        }

        trace!("Added thumbnail to cache: {:?}", path);
        self.cache.insert(path, (texture, Instant::now()));
    }

    pub fn remove(&mut self, path: &Path) {
        debug!("Removing thumbnail: {:?}", path);
        self.cache.remove(path);
        self.pending_loads.remove(path);
        self.failed_loads.remove(path);
    }

    pub fn set_max_size(&mut self, max_size: usize) {
        debug!("Changing maximum cache size: {}", max_size);
        self.max_size = max_size;
        self.cleanup_oversized();
    }

    pub fn set_ttl(&mut self, ttl: Duration) {
        debug!("Changing thumbnail lifetime: {:?}", ttl);
        self.ttl = ttl;
        self.cleanup_expired();
    }

    pub fn clear_cache(&mut self) {
        let cache_size = self.cache.len();
        let failed_size = self.failed_loads.len();

        debug!(
            "Clearing thumbnail cache: {} images, {} problematic files",
            cache_size, failed_size
        );

        self.cache.clear();
        self.failed_loads.clear();

        info!(
            "Thumbnail cache cleared: removed {} images and {} problematic files",
            cache_size, failed_size
        );
    }

    fn cleanup_oversized(&mut self) {
        let before_count = self.cache.len();
        while self.cache.len() > self.max_size {
            if let Some((oldest_key, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, timestamp))| timestamp.elapsed())
            {
                let oldest_key = oldest_key.clone();
                self.cache.remove(&oldest_key);
                trace!("Removed thumbnail due to cache overflow: {:?}", oldest_key);
            } else {
                break;
            }
        }
        let after_count = self.cache.len();
        if before_count != after_count {
            debug!(
                "Cleaning excess thumbnails: removed {}",
                before_count - after_count
            );
        }
    }
}
