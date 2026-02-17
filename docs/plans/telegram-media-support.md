# Plan: Add Image and File Support to RustClaw Telegram Channel

**Author:** UGENT  
**Date:** 2026-02-17  
**Status:** Draft for Review  
**Target:** rustclaw-channel crate

---

## 1. Overview

This plan outlines the implementation of image and file transfer support for the RustClaw Telegram bot channel. Currently, the bot only handles text messages. This enhancement will enable users to:

- **Receive** photos and documents from users via Telegram
- **Send** photos and files back to users
- **Process** images with AI vision capabilities (via MCP tools)

---

## 2. Current State Analysis

### 2.1 What's Working
- Text message handling via `teloxide` v0.17.0
- Command processing (/start, /help, /clear, /tools)
- Long-polling update listener
- Message persistence in SQLite

### 2.2 Current Limitations
```rust
// rustclaw-channel/src/lib.rs:101-103
.branch(
    dptree::filter(|msg: Message| msg.text().is_some()).endpoint(Self::handle_message),
);
```

The handler filters for text only. Non-text messages are silently ignored:
```rust
// rustclaw-channel/src/lib.rs:260-263
let text = match msg.text() {
    Some(t) => t,
    None => return Ok(()),  // <-- Ignores photos/files!
};
```

### 2.3 MessageContent Enum
```rust
// rustclaw-types/src/message.rs
pub enum MessageContent {
    Text(String),
    // Future: Image, File, etc.  <-- Not implemented
}
```

---

## 3. Technical Research Summary

### 3.1 Teloxide Capabilities (v0.17.0)

| Feature | API Method | Status |
|---------|-----------|--------|
| Send Photo | `bot.send_photo()` | ✅ Available |
| Send Document | `bot.send_document()` | ✅ Available |
| Receive Photo | `msg.photo()` | ✅ Available |
| Receive Document | `msg.document()` | ✅ Available |
| Download File | `bot.download_file()` | ✅ Available |
| Get File Info | `bot.get_file()` | ✅ Available |

### 3.2 Key Teloxide Types

```rust
// Sending media
bot.send_photo(chat_id, InputFile::file(path)).caption("...").await?;
bot.send_document(chat_id, InputFile::file(path)).await?;

// Receiving media (from Message type)
msg.photo(): Option<&[PhotoSize]>  // Array of photo sizes
msg.document(): Option<&Document>   // Document metadata

// Downloading files
use teloxide::net::Download;
let file = bot.get_file(file_id).await?;
bot.download_file(&file.path, &mut destination).await?;
```

### 3.3 InputFile Options
```rust
InputFile::file(PathBuf)     // Upload from file path
InputFile::memory(Vec<u8>)    // Upload from memory
InputFile::url(String)        // Send via URL
InputFile::file_id(FileId)    // Re-send existing file
```

---

## 4. Implementation Design

### 4.1 Extended MessageContent Enum

```rust
// rustclaw-types/src/message.rs

pub enum MessageContent {
    Text(String),
    Image(ImageContent),
    Document(DocumentContent),
}

pub struct ImageContent {
    pub file_id: String,
    pub file_unique_id: String,
    pub width: u32,
    pub height: u32,
    pub caption: Option<String>,
    pub local_path: Option<PathBuf>,  // After download
}

pub struct DocumentContent {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<u64>,
    pub caption: Option<String>,
    pub local_path: Option<PathBuf>,  // After download
}
```

### 4.2 New Handler Branches

```rust
// rustclaw-channel/src/lib.rs

let handler = Update::filter_message()
    .branch(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(Self::handle_command),
    )
    .branch(
        dptree::filter(|msg: Message| msg.text().is_some())
            .endpoint(Self::handle_text_message),
    )
    .branch(
        dptree::filter(|msg: Message| msg.photo().is_some())
            .endpoint(Self::handle_photo_message),
    )
    .branch(
        dptree::filter(|msg: Message| msg.document().is_some())
            .endpoint(Self::handle_document_message),
    );
```

### 4.3 File Download Service

```rust
// rustclaw-channel/src/file_service.rs (new file)

pub struct FileService {
    download_dir: PathBuf,
    bot: Bot,
}

impl FileService {
    pub async fn download_photo(&self, photo: &PhotoSize) -> Result<PathBuf> {
        let file = self.bot.get_file(&photo.file_id).await?;
        let local_path = self.download_dir.join(&photo.file_unique_id);
        
        let mut dest = tokio::fs::File::create(&local_path).await?;
        self.bot.download_file(&file.path, &mut dest).await?;
        
        Ok(local_path)
    }
    
    pub async fn download_document(&self, doc: &Document) -> Result<PathBuf> {
        let file = self.bot.get_file(&doc.file_id).await?;
        let filename = doc.file_name.clone().unwrap_or_else(|| 
            format!("document_{}", doc.file_unique_id)
        );
        let local_path = self.download_dir.join(&filename);
        
        let mut dest = tokio::fs::File::create(&local_path).await?;
        self.bot.download_file(&file.path, &mut dest).await?;
        
        Ok(local_path)
    }
}
```

### 4.4 Photo Handler Implementation

```rust
async fn handle_photo_message(
    bot: Bot,
    msg: Message,
    persistence: Arc<RwLock<PersistenceService>>,
    provider: Arc<RwLock<ProviderService>>,
    file_service: Arc<FileService>,
) -> Result<(), teloxide::RequestError> {
    let photos = match msg.photo() {
        Some(p) => p,
        None => return Ok(()),
    };
    
    // Get the largest photo (highest quality)
    let photo = photos.last().ok_or_else(|| anyhow!("No photo"))?;
    
    let chat_id = msg.chat.id;
    
    // Download the photo
    let local_path = file_service.download_photo(photo).await
        .map_err(|e| {
            error!("Failed to download photo: {}", e);
            teloxide::RequestError::RetryAfter(1)
        })?;
    
    // Create image content
    let image_content = ImageContent {
        file_id: photo.file_id.clone(),
        file_unique_id: photo.file_unique_id.clone(),
        width: photo.width,
        height: photo.height,
        caption: msg.caption().map(|s| s.to_string()),
        local_path: Some(local_path.clone()),
    };
    
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let user = User::new(user_id);
    
    let rustclaw_msg = RustClawMessage::new(
        chat_id.0, 
        user, 
        MessageContent::Image(image_content)
    );
    
    // Save and process...
    // Get AI response with image context
    let caption_text = msg.caption().unwrap_or("[Image]");
    
    let response = {
        let provider = provider.read().await;
        provider.complete_with_image(&local_path, caption_text).await
    };
    
    match response {
        Ok(response) => {
            Self::send_message_safe(&bot, chat_id, &response).await?;
        }
        Err(e) => {
            error!("Failed to get AI response: {}", e);
            Self::send_message_safe(&bot, chat_id, &format!("❌ Error: {}", e)).await?;
        }
    }
    
    Ok(())
}
```

### 4.5 Document Handler Implementation

```rust
async fn handle_document_message(
    bot: Bot,
    msg: Message,
    persistence: Arc<RwLock<PersistenceService>>,
    provider: Arc<RwLock<ProviderService>>,
    file_service: Arc<FileService>,
) -> Result<(), teloxide::RequestError> {
    let doc = match msg.document() {
        Some(d) => d,
        None => return Ok(()),
    };
    
    let chat_id = msg.chat.id;
    
    // Download the document
    let local_path = file_service.download_document(doc).await
        .map_err(|e| {
            error!("Failed to download document: {}", e);
            teloxide::RequestError::RetryAfter(1)
        })?;
    
    // Create document content
    let doc_content = DocumentContent {
        file_id: doc.file_id.clone(),
        file_unique_id: doc.file_unique_id.clone(),
        file_name: doc.file_name.clone(),
        mime_type: doc.mime_type.clone(),
        file_size: Some(doc.file_size),
        caption: msg.caption().map(|s| s.to_string()),
        local_path: Some(local_path.clone()),
    };
    
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let user = User::new(user_id);
    
    let rustclaw_msg = RustClawMessage::new(
        chat_id.0, 
        user, 
        MessageContent::Document(doc_content)
    );
    
    // For documents, inform user and process
    let file_info = format!(
        "📄 Received: {}\nSize: {} bytes",
        doc.file_name.as_deref().unwrap_or("Unknown"),
        doc.file_size
    );
    bot.send_message(chat_id, &file_info).await?;
    
    // Process document content based on type
    // ... (read text files, analyze images, etc.)
    
    Ok(())
}
```

### 4.6 Sending Media Back

```rust
async fn send_image_safe(
    bot: &Bot,
    chat_id: ChatId,
    image_path: &Path,
    caption: Option<&str>,
) -> Result<(), teloxide::RequestError> {
    let mut request = bot.send_photo(chat_id, InputFile::file(image_path.to_path_buf()));
    
    if let Some(c) = caption {
        request = request.caption(c);
    }
    
    request.await?;
    Ok(())
}

async fn send_document_safe(
    bot: &Bot,
    chat_id: ChatId,
    file_path: &Path,
    caption: Option<&str>,
) -> Result<(), teloxide::RequestError> {
    let mut request = bot.send_document(chat_id, InputFile::file(file_path.to_path_buf()));
    
    if let Some(c) = caption {
        request = request.caption(c);
    }
    
    request.await?;
    Ok(())
}
```

---

## 5. Configuration Changes

### 5.1 rustclaw.toml Addition

```toml
[files]
# Directory for downloaded files (relative to workspace)
download_dir = "./downloads"

# Maximum file size to download (in MB)
max_download_size_mb = 50

# No auto-cleanup - files kept forever
```

---

## 6. Implementation Tasks

### Phase 1: Core Types (1-2 hours)
- [ ] Extend `MessageContent` enum in `rustclaw-types`
- [ ] Add `ImageContent` and `DocumentContent` structs
- [ ] Update serialization/deserialization

### Phase 2: File Service (2-3 hours)
- [ ] Create `FileService` struct
- [ ] Implement `download_photo()`
- [ ] Implement `download_document()`
- [ ] Add file cleanup logic
- [ ] Handle download errors

### Phase 3: Message Handlers (2-3 hours)
- [ ] Add photo message handler branch
- [ ] Add document message handler branch
- [ ] Integrate with existing `ProviderService`
- [ ] Update persistence layer

### Phase 4: Send Media (1-2 hours)
- [ ] Implement `send_image_safe()`
- [ ] Implement `send_document_safe()`
- [ ] Add to response handling

### Phase 5: Configuration (1 hour)
- [ ] Add `[files]` config section
- [ ] Parse and validate config
- [ ] Create download directory on startup

### Phase 6: Testing (2 hours)
- [ ] Unit tests for file service
- [ ] Integration tests for handlers
- [ ] Manual testing with Telegram

---

## 7. Security Considerations

1. **File Size Limits**: Enforce maximum download size
2. **File Type Validation**: Check MIME types before processing
3. **Path Traversal**: Sanitize file names to prevent directory traversal
4. **Cleanup**: Auto-delete downloaded files after processing
5. **Quota**: Consider per-user download quotas

```rust
// Security helpers
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

fn is_allowed_file_type(mime: &Option<String>) -> bool {
    match mime {
        Some(m) => {
            m.starts_with("image/")
                || m.starts_with("text/")
                || m == "application/pdf"
                || m == "application/json"
        }
        None => false,
    }
}
```

---

## 8. Dependencies

No new dependencies required. All functionality is available in:
- `teloxide` v0.17.0 (already used)
- `tokio` (already used for async)
- `serde_json` (already used)

---

## 9. Future Enhancements (Out of Scope)

- Voice message support
- Video processing
- Audio file handling
- Sticker reactions
- Inline query results with media
- Streaming large file downloads

---

## 10. Design Decisions (Approved)

| Question | Decision |
|----------|----------|
| Download directory | Workspace subfolder `./downloads` |
| Image AI integration | LLM decides whether to use vision or MCP tools |
| File retention | Keep files forever (no auto-cleanup) |
| Response format | Send images/files back via Telegram when generated |
| Multi-file handling | Process via queue, one at a time |

---

**Status: APPROVED - Ready for implementation**
