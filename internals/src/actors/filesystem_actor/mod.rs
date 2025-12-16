use actix::prelude::*;
use log::debug;
use file_format::{FileFormat, Kind};

use crate::messages::filesystem::{
    ReadFileContent, WriteFileContent, CreateFile, CreateDirectory, DeleteEntry, RenameEntry, PathExists,
    BuildFileTree, LoadDirectoryContents, StartFileWatcher, StopFileWatcher, StopAllFileWatchers,
};

pub struct FilesystemActor;

impl Default for FilesystemActor {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemActor {
    pub fn new() -> Self {
        Self
    }
}

impl Actor for FilesystemActor {
    type Context = Context<Self>;
}

impl Handler<ReadFileContent> for FilesystemActor {
    type Result = Result<String, String>;

    fn handle(&mut self, msg: ReadFileContent, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Read file: {}", msg.path);
        
        // First, read the file as bytes to check if it's binary
        let file_bytes = match std::fs::read(&msg.path) {
            Ok(bytes) => bytes,
            Err(e) => return Err(e.to_string()),
        };
        
        // Handle empty files - they should be treated as text files
        if file_bytes.is_empty() {
            debug!("[FS] File {} is empty, treating as text file", msg.path);
            return Ok(String::new());
        }
        
        // Check if the file is binary using file-format crate
        let file_format = FileFormat::from_bytes(&file_bytes);
        
        // Check if the file format is text-based using the Kind enum
        let is_text_based = match file_format.kind() {
            Kind::Other => {
                // Check if it's a known text format that falls under "Other"
                matches!(file_format, 
                    FileFormat::PlainText |
                    FileFormat::ExtensibleMarkupLanguage |
                    FileFormat::Atom |
                    FileFormat::ReallySimpleSyndication |
                    FileFormat::JsonFeed |
                    FileFormat::SimpleObjectAccessProtocol |
                    FileFormat::XmlLocalizationInterchangeFileFormat |
                    FileFormat::Musicxml |
                    FileFormat::Icalendar |
                    FileFormat::Vcalendar |
                    FileFormat::Vcard |
                    FileFormat::SubripText |
                    FileFormat::TimedTextMarkupLanguage |
                    FileFormat::UniversalSubtitleFormat |
                    FileFormat::WebVideoTextTracks |
                    FileFormat::TiledMapXml |
                    FileFormat::TiledTilesetXml
                )
            },
            Kind::Document => {
                // Some document formats are text-based (like XML-based ones)
                matches!(file_format, 
                    FileFormat::ExtensibleMarkupLanguage |
                    FileFormat::Atom |
                    FileFormat::ReallySimpleSyndication |
                    FileFormat::JsonFeed |
                    FileFormat::SimpleObjectAccessProtocol |
                    FileFormat::XmlLocalizationInterchangeFileFormat |
                    FileFormat::Musicxml |
                    FileFormat::Icalendar |
                    FileFormat::Vcalendar |
                    FileFormat::Vcard |
                    FileFormat::SubripText |
                    FileFormat::TimedTextMarkupLanguage |
                    FileFormat::UniversalSubtitleFormat |
                    FileFormat::WebVideoTextTracks |
                    FileFormat::TiledMapXml |
                    FileFormat::TiledTilesetXml
                )
            },
            _ => false, // Image, Audio, Video, Archive, etc. are binary
        };
        
        // If it's not text-based, return an error
        if !is_text_based {
            debug!("[FS] File {} is binary format: {:?} (kind: {:?})", msg.path, file_format, file_format.kind());
            return Err("Binary file cannot be displayed as text".to_string());
        }
        
        // For text files, try to read as string
        // This will fail gracefully for binary files that aren't recognized
        String::from_utf8(file_bytes).map_err(|_| {
            debug!("[FS] File {} contains invalid UTF-8, treating as binary", msg.path);
            "Binary file cannot be displayed as text".to_string()
        })
    }
}

impl Handler<WriteFileContent> for FilesystemActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WriteFileContent, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Write file: {}", msg.path);
        std::fs::write(&msg.path, msg.content).map_err(|e| e.to_string())
    }
}

impl Handler<CreateFile> for FilesystemActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: CreateFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Create file: {}", msg.path);
        if std::path::Path::new(&msg.path).exists() {
            return Err("File already exists".to_string());
        }
        std::fs::File::create(&msg.path).map_err(|e| e.to_string()).map(|_| ())
    }
}

impl Handler<CreateDirectory> for FilesystemActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: CreateDirectory, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Create directory: {}", msg.path);
        if std::path::Path::new(&msg.path).exists() {
            return Err("Folder already exists".to_string());
        }
        std::fs::create_dir_all(&msg.path).map_err(|e| e.to_string())
    }
}

impl Handler<DeleteEntry> for FilesystemActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: DeleteEntry, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Delete entry: {}", msg.path);
        let path = std::path::Path::new(&msg.path);
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(|e| e.to_string())
        } else if path.is_file() {
            std::fs::remove_file(path).map_err(|e| e.to_string())
        } else {
            Err("Item does not exist or is not a file/directory".to_string())
        }
    }
}

impl Handler<RenameEntry> for FilesystemActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RenameEntry, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Rename: {} -> {}", msg.old_path, msg.new_path);
        let old_path = std::path::Path::new(&msg.old_path);
        let new_path = std::path::Path::new(&msg.new_path);
        if !old_path.exists() {
            return Err("Item does not exist".to_string());
        }
        if new_path.exists() {
            return Err("Target path already exists".to_string());
        }
        std::fs::rename(old_path, new_path).map_err(|e| e.to_string())
    }
}

impl Handler<PathExists> for FilesystemActor {
    type Result = Result<bool, String>;

    fn handle(&mut self, msg: PathExists, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(std::path::Path::new(&msg.path).exists())
    }
}

impl Handler<BuildFileTree> for FilesystemActor {
    type Result = Result<serde_json::Value, String>;

    fn handle(&mut self, msg: BuildFileTree, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Build tree: {}", msg.root_path);
        let path = std::path::PathBuf::from(&msg.root_path);
        
        debug!("[FS] BuildFileTree: Path exists: {}, is_dir: {}", 
            path.exists(), 
            path.is_dir()
        );
        
        if !path.is_dir() {
            debug!("[FS] BuildFileTree: Provided path is not a directory: {:?}", path);
            return Err("Provided path is not a directory".to_string());
        }
        
        // Use async build_tree and convert to JSON synchronously by blocking on it
        let fut = crate::services::build_tree(&path);
        let file_node = futures::executor::block_on(fut)?;
        
        // Log the structure of the returned file node
        if let Some(ref children) = file_node.children {
            debug!("[FS] BuildFileTree: Returning {} top-level items", children.len());
            let dir_names: Vec<&String> = children
                .iter()
                .filter(|c| c.is_directory)
                .map(|c| &c.name)
                .collect();
            debug!("[FS] BuildFileTree: Top-level directories: {:?}", dir_names);
            
            let has_demo = children.iter().any(|c| c.is_directory && c.name == "demo");
            debug!("[FS] BuildFileTree: Demo folder in result: {}", has_demo);
        } else {
            debug!("[FS] BuildFileTree: No children returned");
        }
        
        serde_json::to_value(file_node).map_err(|e| e.to_string())
    }
}

impl Handler<LoadDirectoryContents> for FilesystemActor {
    type Result = Result<serde_json::Value, String>;

    fn handle(&mut self, msg: LoadDirectoryContents, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Load directory contents: {}", msg.path);
        let path = std::path::PathBuf::from(&msg.path);
        if !path.is_dir() {
            return Err("Provided path is not a directory".to_string());
        }
        
        // Use async load_directory_contents and convert to JSON synchronously by blocking on it
        let fut = crate::services::load_directory_contents(&path);
        let contents = futures::executor::block_on(fut)?;
        serde_json::to_value(contents).map_err(|e| e.to_string())
    }
}

// File watching handlers - these delegate to the FileWatcherActor
impl Handler<StartFileWatcher> for FilesystemActor {
    type Result = Result<String, String>;
    
    fn handle(&mut self, msg: StartFileWatcher, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Start file watcher: {}", msg.path);
        // This should be handled by the FileWatcherActor, but we'll implement a placeholder
        // In a real implementation, we'd forward this to the FileWatcherActor
        Err("File watching not yet implemented in FilesystemActor".to_string())
    }
}

impl Handler<StopFileWatcher> for FilesystemActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: StopFileWatcher, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Stop file watcher: {}", msg.watcher_id);
        // This should be handled by the FileWatcherActor, but we'll implement a placeholder
        Err("File watching not yet implemented in FilesystemActor".to_string())
    }
}

impl Handler<StopAllFileWatchers> for FilesystemActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopAllFileWatchers, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[FS] Stop all file watchers");
        // This should be handled by the FileWatcherActor, but we'll implement a placeholder
        Err("File watching not yet implemented in FilesystemActor".to_string())
    }
}


