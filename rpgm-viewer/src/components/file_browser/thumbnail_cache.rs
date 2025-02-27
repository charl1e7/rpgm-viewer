use log::{debug, error, info, trace};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    thread,
    time::SystemTime,
};

use super::file_entry::FileEntry;

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
    cache: HashMap<PathBuf, (egui::TextureHandle, SystemTime)>,
    pending_loads: HashSet<PathBuf>,
    failed_loads: HashSet<PathBuf>,
    channels: Option<Arc<ThreadChannels>>,
    worker_running: bool,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        info!("Creating new ThumbnailCache");

        let (task_tx, task_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        Self::start_worker_thread(task_rx, result_tx.clone());

        let channels = Arc::new(ThreadChannels {
            sender: task_tx,
            receiver: result_rx,
        });

        Self {
            cache: HashMap::new(),
            pending_loads: HashSet::new(),
            failed_loads: HashSet::new(),
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
            while let Ok(task) = task_rx.recv() {
                debug!("Received thumbnail processing task: {:?}", task.path);
                let result = Self::process_thumbnail_task(task);
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

            self.channels = Some(Arc::new(ThreadChannels {
                sender: task_tx,
                receiver: result_rx,
            }));
            self.worker_running = true;
        }
    }

    pub fn request_thumbnail(
        &mut self,
        path: &Path,
        decrypter: &rpgm_enc::Decrypter,
        compression_size: u32,
    ) {
        if self.is_pending(path) || self.failed_loads.contains(path) {
            return;
        }

        self.ensure_initialized();

        if let Some(channels) = &self.channels {
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
            }
        }
    }

    pub fn process_results(&mut self, ctx: &egui::Context) -> Vec<(PathBuf, egui::TextureHandle)> {
        let mut loaded_thumbnails = Vec::new();

        self.ensure_initialized();

        let channels = self
            .channels
            .clone()
            .expect("Channels should be initialized");
        let receiver = &channels.receiver;

        let mut results = Vec::new();
        while let Ok(result) = receiver.try_recv() {
            debug!("Received thumbnail loading result: {:?}", result.path);
            results.push(result);
        }

        for result in results {
            if let Some(texture_data) = result.texture_data {
                let (raw_data, dimensions) = texture_data;
                let texture = ctx.load_texture(
                    format!(
                        "thumb_{}",
                        result.path.file_name().unwrap().to_string_lossy()
                    ),
                    egui::ColorImage::from_rgb([dimensions[0], dimensions[1]], &raw_data),
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Linear,
                        minification: egui::TextureFilter::Linear,
                        ..Default::default()
                    },
                );
                let modified_time = std::fs::metadata(&result.path)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::now());
                self.insert(result.path.clone(), texture.clone(), modified_time);
                loaded_thumbnails.push((result.path.clone(), texture));
            } else {
                self.failed_loads.insert(result.path.clone());
            }
            self.unmark_pending(&result.path);
        }

        loaded_thumbnails
    }

    pub fn get(&mut self, path: &Path) -> Option<egui::TextureHandle> {
        if let Some((texture, modified_time)) = self.cache.get(path) {
            if let Ok(current_modified) = std::fs::metadata(path).and_then(|m| m.modified()) {
                if *modified_time == current_modified {
                    return Some(texture.clone());
                } else {
                    self.cache.remove(path);
                }
            } else {
                self.cache.remove(path);
            }
        }
        None
    }

    pub fn update_cache(&mut self, root: &Path) {
        let mut to_remove = Vec::new();
        for (path, (_, modified_time)) in self.cache.iter() {
            match std::fs::metadata(path) {
                Ok(metadata) => {
                    if let Ok(current_modified) = metadata.modified() {
                        if *modified_time != current_modified {
                            to_remove.push(path.clone());
                        }
                    }
                }
                Err(_) => {
                    to_remove.push(path.clone());
                }
            }
        }

        for path in to_remove {
            self.cache.remove(&path);
            info!(
                "Removed outdated or deleted thumbnail from cache: {:?}",
                path
            );
        }

        let entries = FileEntry::recursive_collect_all_entries_flat(root, 0);
        for entry in entries {
            if !entry.is_folder
                && self.get(&entry.path).is_none()
                && !self.is_pending(&entry.path)
                && !self.is_failed(&entry.path)
            {
                debug!("Thumbnail missing for: {:?}", entry.path);
            }
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

    pub fn insert(
        &mut self,
        path: PathBuf,
        texture: egui::TextureHandle,
        modified_time: SystemTime,
    ) {
        debug!("Added thumbnail to cache: {:?}", path);
        self.cache.insert(path, (texture, modified_time));
    }

    pub fn remove(&mut self, path: &Path) {
        debug!("Removing thumbnail: {:?}", path);
        self.cache.remove(path);
        self.pending_loads.remove(path);
        self.failed_loads.remove(path);
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
}
