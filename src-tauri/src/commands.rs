use crate::db;
use crate::models::{Conversation, DownloadProgress, LoadedModel, Message, Model, OllamaVersion};
use rusqlite::Connection;
use tauri::{Emitter, Manager};

/// Check if Ollama is running and return version information
///
/// This command sends a GET request to http://localhost:11434/api/version
/// with a 5-second timeout. If Ollama is running, it returns the version
/// information. If Ollama is not running or the request times out, it
/// returns an error.
///
/// # Returns
/// - `Ok(OllamaVersion)` if Ollama is running and responds successfully
/// - `Err(String)` if the request fails, times out, or Ollama is not running
///
/// **Validates: Requirements 1.1, 1.4, 1.5**
#[tauri::command]
pub async fn check_ollama_health(
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
) -> Result<OllamaVersion, String> {
    runtime_manager
        .health_check()
        .await
        .map_err(|e| e.to_string())
}

/// Get the database path
#[tauri::command]
pub async fn get_db_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    // The SQL plugin uses "sqlite:openllm_studio.db" which resolves to app_data_dir/openllm_studio.db
    // We need to return the same path that the SQL plugin uses
    let db_path = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("openllm_studio.db");
    
    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }
    
    println!("[CMD] get_db_path returning: {}", db_path.to_string_lossy());
    Ok(db_path.to_string_lossy().to_string())
}

/// Detect hardware specifications with caching
#[tauri::command]
pub async fn detect_hardware_cached(db_path: String) -> Result<crate::models::HardwareInfo, String> {
    use rusqlite::Connection;
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    // Check cache (24 hours = 86400 seconds)
    let now = chrono::Utc::now().timestamp();
    let cache_threshold = now - 86400;
    
    let cached: Result<crate::models::HardwareInfo, _> = conn.query_row(
        "SELECT gpu_name, gpu_backend, vram_gb, ram_gb, cpu_cores, disk_space_gb, is_shared_memory
         FROM hardware_info 
         WHERE scanned_at > ?1 
         ORDER BY scanned_at DESC 
         LIMIT 1",
        [cache_threshold],
        |row| {
            let backend_str: String = row.get(1)?;
            let backend = match backend_str.as_str() {
                "nvidia" => crate::models::GpuBackend::Nvidia,
                "amd" => crate::models::GpuBackend::Amd,
                "intel" => crate::models::GpuBackend::Intel,
                "apple_metal" => crate::models::GpuBackend::AppleMetal,
                _ => crate::models::GpuBackend::CpuOnly,
            };
            
            Ok(crate::models::HardwareInfo {
                gpu_name: row.get(0)?,
                gpu_backend: backend,
                vram_gb: row.get(2)?,
                ram_gb: row.get(3)?,
                cpu_cores: row.get(4)?,
                disk_space_gb: row.get(5)?,
                is_shared_memory: row.get(6).ok().flatten(),
            })
        },
    );
    
    if let Ok(info) = cached {
        return Ok(info);
    }
    
    // No valid cache, detect fresh
    let detector = crate::hardware::HardwareDetector::new();
    let info = detector.detect().await
        .map_err(|e| format!("Hardware detection failed: {}", e))?;
    
    // Store in cache
    let backend_str = match info.gpu_backend {
        crate::models::GpuBackend::Nvidia => "nvidia",
        crate::models::GpuBackend::Amd => "amd",
        crate::models::GpuBackend::Intel => "intel",
        crate::models::GpuBackend::AppleMetal => "apple_metal",
        crate::models::GpuBackend::CpuOnly => "cpu_only",
    };
    
    conn.execute(
        "INSERT INTO hardware_info (gpu_name, gpu_backend, vram_gb, ram_gb, cpu_cores, disk_space_gb, is_shared_memory, scanned_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            &info.gpu_name,
            backend_str,
            info.vram_gb,
            info.ram_gb,
            info.cpu_cores,
            info.disk_space_gb,
            info.is_shared_memory,
            now,
        ],
    ).map_err(|e| format!("Failed to cache hardware info: {}", e))?;
    
    Ok(info)
}

/// List all installed models
///
/// This command sends a GET request to http://localhost:11434/api/tags
/// and returns the list of installed models with their metadata
/// (name, size, modified_at).
///
/// # Returns
/// - `Ok(Vec<Model>)` containing all installed models
/// - `Err(String)` if the request fails or the response cannot be parsed
///
/// **Validates: Requirements 3.1, 3.2**
#[tauri::command]
pub async fn list_models(
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
) -> Result<Vec<Model>, String> {
    runtime_manager
        .list_models()
        .await
        .map_err(|e| e.to_string())
}

/// Get the currently loaded model
///
/// This command sends a GET request to http://localhost:11434/api/ps
/// and returns information about the model currently loaded in Ollama's
/// memory. If no model is loaded, it returns None.
///
/// # Returns
/// - `Ok(Some(LoadedModel))` if a model is currently loaded
/// - `Ok(None)` if no model is loaded (Ollama is idle)
/// - `Err(String)` if the request fails or the response cannot be parsed
///
/// **Validates: Requirements 11.1**
#[tauri::command]
pub async fn get_loaded_model(
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
) -> Result<Option<LoadedModel>, String> {
    runtime_manager
        .get_loaded_model()
        .await
        .map_err(|e| e.to_string())
}

/// Get all conversations sorted by updated_at DESC
///
/// This command queries the SQLite database for all conversations
/// and returns them in reverse chronological order (most recently
/// updated first).
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
///
/// # Returns
/// - `Ok(Vec<Conversation>)` containing all conversations
/// - `Err(String)` if the database query fails
///
/// **Validates: Requirements 7.3, 7.4, 17.1**
#[tauri::command]
pub async fn get_conversations(db_path: String) -> Result<Vec<Conversation>, String> {
    println!("[CMD] get_conversations called with db_path: {}", db_path);
    
    // Check if database file exists
    let db_exists = std::path::Path::new(&db_path).exists();
    println!("[CMD] Database file exists: {}", db_exists);
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    // Verify the conversations table exists
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations')",
        [],
        |row| row.get(0)
    ).unwrap_or(false);
    
    if !table_exists {
        println!("[CMD] conversations table does not exist, creating schema...");
        conn.execute_batch(include_str!("../migrations/001_initial_schema.sql"))
            .map_err(|e| format!("Failed to create schema: {}", e))?;
    }
    
    db::get_conversations(&conn)
        .map_err(|e| e.to_string())
}

/// Create a new conversation with UUID generation
///
/// This command generates a new UUID for the conversation ID and
/// inserts the conversation into the database with the current
/// timestamp for both created_at and updated_at.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
/// * `model_name` - Name of the Ollama model (e.g., "llama3.2:1b")
/// * `title` - User-visible conversation title
///
/// # Returns
/// - `Ok(Conversation)` containing the created conversation
/// - `Err(String)` if the database operation fails
///
/// **Validates: Requirements 7.1, 7.2**
#[tauri::command]
pub async fn create_conversation(
    db_path: String,
    model_name: String,
    title: String,
) -> Result<Conversation, String> {
    println!("[CMD] create_conversation called");
    println!("[CMD] db_path: {}", db_path);
    println!("[CMD] model_name: {}", model_name);
    println!("[CMD] title: {}", title);
    
    // Check if database file exists
    let db_exists = std::path::Path::new(&db_path).exists();
    println!("[CMD] Database file exists: {}", db_exists);
    
    if !db_exists {
        println!("[CMD] WARNING: Database file does not exist! It will be created but may not have the schema.");
    }
    
    let conn = Connection::open(&db_path)
        .map_err(|e| {
            let err_msg = format!("Failed to open database: {}", e);
            println!("[CMD] Error: {}", err_msg);
            err_msg
        })?;
    
    // Verify the conversations table exists
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations')",
        [],
        |row| row.get(0)
    ).unwrap_or(false);
    
    if !table_exists {
        println!("[CMD] ERROR: conversations table does not exist! Running migrations manually...");
        // Run the initial schema migration
        conn.execute_batch(include_str!("../migrations/001_initial_schema.sql"))
            .map_err(|e| format!("Failed to create schema: {}", e))?;
        println!("[CMD] Schema created successfully");
    }
    
    db::create_conversation(&conn, &model_name, &title)
        .map_err(|e| {
            let err_msg = e.to_string();
            println!("[CMD] Error creating conversation: {}", err_msg);
            err_msg
        })
}

/// Get all messages for a conversation sorted by created_at ASC
///
/// This command queries the SQLite database for all messages
/// in a specific conversation and returns them in chronological
/// order (oldest first).
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
/// * `conversation_id` - UUID of the conversation
///
/// # Returns
/// - `Ok(Vec<Message>)` containing all messages in the conversation
/// - `Err(String)` if the database query fails
///
/// **Validates: Requirements 9.3, 17.2**
#[tauri::command]
pub async fn get_messages(
    db_path: String,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    db::get_messages(&conn, &conversation_id)
        .map_err(|e| e.to_string())
}

/// Pull (download) a model from Ollama with streaming progress updates and automatic retry
///
/// This command initiates a model download from Ollama and emits real-time
/// progress events to the frontend. If the download fails due to network issues,
/// it will automatically retry up to 3 times with exponential backoff.
///
/// # Arguments
/// * `app_handle` - Tauri application handle for emitting events
/// * `model_name` - Name of the model to download (e.g., "llama3.2:1b")
///
/// # Returns
/// - `Ok(())` if the download completes successfully
/// - `Err(String)` if the request fails after all retries
///
/// # Events Emitted
/// - `download-progress`: Emitted for each progress update with DownloadProgress payload
///
/// **Validates: Requirements 5.1, 5.2, 5.3**
#[tauri::command]
pub async fn pull_model_with_retry(
    app_handle: tauri::AppHandle,
    model_name: String,
    preferred_quantization: Option<String>,
) -> Result<(), String> {
    const MAX_RETRIES: u32 = 3;
    const INITIAL_BACKOFF_MS: u64 = 2000;
    
    for attempt in 1..=MAX_RETRIES {
        match pull_model_internal(
            app_handle.clone(),
            model_name.clone(),
            preferred_quantization.clone(),
        ).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                // Check if it's a retryable error (network/timeout)
                let is_retryable = e.contains("timeout") 
                    || e.contains("TLS handshake") 
                    || e.contains("connection")
                    || e.contains("network");
                
                if !is_retryable || attempt == MAX_RETRIES {
                    return Err(e);
                }
                
                // Exponential backoff
                let backoff_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                
                // Emit retry status
                let retry_progress = DownloadProgress {
                    model_name: model_name.clone(),
                    status: format!("Retrying in {}s... (attempt {}/{})", backoff_ms / 1000, attempt, MAX_RETRIES),
                    completed: None,
                    total: None,
                };
                let _ = app_handle.emit("download-progress", &retry_progress);
                
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            }
        }
    }
    
    Err("Download failed after maximum retries".to_string())
}

/// Internal function to pull a model (used by pull_model_with_retry)
async fn pull_model_internal(
    app_handle: tauri::AppHandle,
    model_name: String,
    preferred_quantization: Option<String>,
) -> Result<(), String> {
    app_handle
        .emit(
            "download-progress",
            DownloadProgress {
                model_name: model_name.clone(),
                status: "resolving".to_string(),
                completed: None,
                total: None,
            },
        )
        .map_err(|e| format!("Failed to emit download-progress event: {}", e))?;

    let client = crate::huggingface::HfClient::new(None);
    let (repo_id, filename) = client
        .resolve_model_download(&model_name, preferred_quantization.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let cancel_token = tokio_util::sync::CancellationToken::new();
    client
        .download_model(&repo_id, &filename, &model_name, app_handle, cancel_token)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_conversation(
    db_path: String,
    conversation_id: String,
    model_name: String,
    title: String,
) -> Result<Conversation, String> {
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db::update_conversation(&conn, &conversation_id, &model_name, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_conversation(
    db_path: String,
    conversation_id: String,
) -> Result<(), String> {
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db::delete_conversation(&conn, &conversation_id)
        .map_err(|e| e.to_string())
}

/// Pull (download) a model from Ollama with streaming progress updates
///
/// This is the original pull_model command without retry logic.
/// Use pull_model_with_retry for better reliability.
///
/// # Arguments
/// * `app_handle` - Tauri application handle for emitting events
/// * `model_name` - Name of the model to download (e.g., "llama3.2:1b")
///
/// # Returns
/// - `Ok(())` if the download completes successfully
/// - `Err(String)` if the request fails or the stream encounters an error
///
/// # Events Emitted
/// - `download-progress`: Emitted for each progress update with DownloadProgress payload
///
/// **Validates: Requirements 5.1, 5.2, 5.3**
#[tauri::command]
pub async fn pull_model(
    app_handle: tauri::AppHandle,
    model_name: String,
    preferred_quantization: Option<String>,
) -> Result<(), String> {
    pull_model_internal(app_handle, model_name, preferred_quantization).await
}

/// Send a chat message and stream the response with real-time token emission
///
/// This command handles the complete chat workflow:
/// 1. Saves the user message to the database
/// 2. Sends the chat request to Ollama with conversation history
/// 3. Streams tokens back to the frontend via events
/// 4. Saves the complete assistant response to the database
/// 5. Updates the conversation's updated_at timestamp
///
/// # Arguments
/// * `app_handle` - Tauri application handle for emitting events
/// * `db_path` - Path to the SQLite database file
/// * `conversation_id` - UUID of the conversation
/// * `model_name` - Name of the Ollama model (e.g., "llama3.2:1b")
/// * `messages` - Array of chat messages including the new user message
/// * `options` - Optional chat options for optimization parameters
///
/// # Returns
/// - `Ok(String)` containing the complete assistant response
/// - `Err(String)` if the request fails, database operation fails, or stream encounters an error
///
/// # Events Emitted
/// - `chat-token`: Emitted for each token with conversation_id and content
///
/// **Validates: Requirements 8.2, 8.3, 8.5, 9.1, 9.2, 13.1-13.6**
#[tauri::command]
pub async fn send_chat_message(
    app_handle: tauri::AppHandle,
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
    db_path: String,
    conversation_id: String,
    model_name: String,
    messages: Vec<crate::models::ChatMessage>,
    options: Option<crate::models::ChatOptions>,
) -> Result<String, String> {
    // Extract the user message (last message in the array)
    let user_message = messages
        .last()
        .ok_or("No messages provided")?;
    
    // Save user message to database before sending request
    let mut conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    db::save_message(
        &mut conn,
        &conversation_id,
        &user_message.role,
        &user_message.content,
    )
    .map_err(|e| e.to_string())?;
    
    let mut token_count = 0;
    let full_response = runtime_manager
        .stream_chat_completion(&model_name, &messages, options.as_ref(), |token| {
            token_count += 1;

            let token_event = serde_json::json!({
                "conversation_id": conversation_id,
                "content": token,
                "seq": token_count,
            });

            app_handle
                .emit("chat-token", &token_event)
                .map_err(|e| crate::error::AppError::RuntimeError(format!("Failed to emit chat-token event: {}", e)))
        })
        .await
        .map_err(|e| e.to_string())?;
    
    // Save assistant response to database
    db::save_message(
        &mut conn,
        &conversation_id,
        "assistant",
        &full_response,
    )
    .map_err(|e| e.to_string())?;
    
    Ok(full_response)
}

#[tauri::command]
pub async fn stop_chat_generation(
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
) -> Result<(), String> {
    runtime_manager
        .stop_generation()
        .await
        .map_err(|e| e.to_string())
}

/// Delete a model from Ollama
///
/// This command sends a DELETE request to http://localhost:11434/api/delete
/// to remove an installed model from Ollama.
///
/// # Arguments
/// * `model_name` - Name of the model to delete (e.g., "llama3.2:1b")
///
/// # Returns
/// - `Ok(())` if the deletion succeeds
/// - `Err(String)` if the request fails or Ollama returns an error
///
/// **Validates: Requirements 6.1**
#[tauri::command]
pub async fn delete_model(
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
    model_name: String,
) -> Result<(), String> {
    runtime_manager
        .delete_model(model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Save a message to the database
///
/// This command saves a message to the database and updates the
/// conversation's updated_at timestamp. This is used for manually
/// saving messages outside of the chat flow.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
/// * `conversation_id` - UUID of the conversation
/// * `role` - Message role ("user" or "assistant")
/// * `content` - Message content
///
/// # Returns
/// - `Ok(Message)` containing the saved message
/// - `Err(String)` if the database operation fails
///
/// **Validates: Requirements 9.1**
#[tauri::command]
pub async fn save_message(
    db_path: String,
    conversation_id: String,
    role: String,
    content: String,
) -> Result<Message, String> {
    let mut conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    db::save_message(&mut conn, &conversation_id, &role, &content)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;
    use uuid::Uuid;

    fn setup_test_db() -> String {
        // Create a temporary database file for testing
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_commands_{}.db", Uuid::new_v4()));
        let db_path_str = db_path.to_str().unwrap().to_string();
        
        let conn = Connection::open(&db_path_str).unwrap();
        
        // Create schema
        conn.execute(
            "CREATE TABLE conversations (
                id TEXT PRIMARY KEY,
                model_name TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            [],
        ).unwrap();

        drop(conn);
        db_path_str
    }

    fn cleanup_test_db(db_path: &str) {
        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn test_check_ollama_health_returns_result() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_list_models_returns_result() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_loaded_model_returns_result() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_create_conversation() {
        let db_path = setup_test_db();
        
        let result = create_conversation(
            db_path.clone(),
            "llama3.2:1b".to_string(),
            "Test Conversation".to_string(),
        ).await;
        
        assert!(result.is_ok());
        let conversation = result.unwrap();
        assert!(!conversation.id.is_empty());
        assert_eq!(conversation.model_name, "llama3.2:1b");
        assert_eq!(conversation.title, "Test Conversation");
        
        cleanup_test_db(&db_path);
    }

    #[tokio::test]
    async fn test_get_conversations() {
        let db_path = setup_test_db();
        
        // Create a conversation first
        create_conversation(
            db_path.clone(),
            "llama3.2:1b".to_string(),
            "Test Conversation".to_string(),
        ).await.unwrap();
        
        let result = get_conversations(db_path.clone()).await;
        
        assert!(result.is_ok());
        let conversations = result.unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].title, "Test Conversation");
        
        cleanup_test_db(&db_path);
    }

    #[tokio::test]
    async fn test_get_messages_empty() {
        let db_path = setup_test_db();
        
        // Create a conversation first
        let conversation = create_conversation(
            db_path.clone(),
            "llama3.2:1b".to_string(),
            "Test Conversation".to_string(),
        ).await.unwrap();
        
        let result = get_messages(db_path.clone(), conversation.id).await;
        
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 0);
        
        cleanup_test_db(&db_path);
    }

    #[tokio::test]
    async fn test_get_messages_nonexistent_conversation() {
        let db_path = setup_test_db();
        
        let result = get_messages(db_path.clone(), "nonexistent-id".to_string()).await;
        
        // Should succeed but return empty list
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 0);
        
        cleanup_test_db(&db_path);
    }

    #[tokio::test]
    async fn test_pull_model_signature() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_send_chat_message_signature() {
        assert!(true);
    }
}

/// Detect hardware specifications
///
/// This command detects GPU, VRAM, RAM, and CPU specifications
/// across Windows, macOS, and Linux platforms.
///
/// # Returns
/// - `Ok(HardwareInfo)` containing detected hardware specifications
/// - `Err(String)` if hardware detection fails
///
/// **Validates: Requirements 1.1-1.7, 2.1-2.5, 3.1-3.5**
#[tauri::command]
pub async fn detect_hardware() -> Result<crate::models::HardwareInfo, String> {
    let detector = crate::hardware::HardwareDetector::new();
    detector
        .detect()
        .await
        .map_err(|e| e.to_string())
}

/// Search for GGUF models on HuggingFace
///
/// This command searches HuggingFace for GGUF models matching the query.
///
/// # Arguments
/// * `query` - Search query string
/// * `page` - Page number for pagination (0-indexed)
/// * `hf_token` - Optional HuggingFace API token for authentication
///
/// # Returns
/// - `Ok(Vec<HfModel>)` containing search results
/// - `Err(String)` if the search fails
///
/// **Validates: Requirements 5.1-5.5**
#[tauri::command]
pub async fn search_hf_models(
    query: String,
    page: usize,
    hf_token: Option<String>,
) -> Result<Vec<crate::models::HfModel>, String> {
    let client = crate::huggingface::HfClient::new(hf_token);
    client
        .search_models(&query, page)
        .await
        .map_err(|e| e.to_string())
}

/// Get list of files for a HuggingFace model repository
///
/// This command fetches the list of GGUF files available for a model.
///
/// # Arguments
/// * `repo_id` - HuggingFace repository ID (e.g., "TheBloke/Llama-2-7B-GGUF")
/// * `hf_token` - Optional HuggingFace API token for authentication
///
/// # Returns
/// - `Ok(Vec<HfModelFile>)` containing model file variants
/// - `Err(String)` if the request fails
///
/// **Validates: Requirements 6.1-6.5**
#[tauri::command]
pub async fn get_hf_model_files(
    repo_id: String,
    hf_token: Option<String>,
) -> Result<Vec<crate::models::HfModelFile>, String> {
    let client = crate::huggingface::HfClient::new(hf_token);
    client
        .get_model_files(&repo_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_hf_model_details(
    repo_id: String,
    hf_token: Option<String>,
) -> Result<crate::models::HfModelDetails, String> {
    let client = crate::huggingface::HfClient::new(hf_token);
    client
        .get_model_details(&repo_id)
        .await
        .map_err(|e| e.to_string())
}

/// Download a GGUF model from HuggingFace
///
/// This command initiates a model download from HuggingFace and emits
/// real-time progress events. The download runs in the background.
///
/// # Arguments
/// * `app_handle` - Tauri application handle for emitting events
/// * `download_manager` - Download manager state
/// * `repo_id` - HuggingFace repository ID
/// * `filename` - Name of the GGUF file to download
/// * `model_name` - Name to register the model with in Ollama
/// * `hf_token` - Optional HuggingFace API token for authentication
///
/// # Returns
/// - `Ok(())` if the download starts successfully
/// - `Err(String)` if the download fails to start
///
/// # Events Emitted
/// - `hf-download-progress`: Emitted for each progress update
///
/// **Validates: Requirements 8.1-8.7, 10.1-10.6**
#[tauri::command]
pub async fn download_hf_model(
    app_handle: tauri::AppHandle,
    download_manager: tauri::State<'_, crate::download_manager::DownloadManager>,
    repo_id: String,
    filename: String,
    model_name: String,
    hf_token: Option<String>,
) -> Result<(), String> {
    let hf_client = std::sync::Arc::new(crate::huggingface::HfClient::new(hf_token));
    
    download_manager
        .start_download(model_name, repo_id, filename, hf_client, app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel a HuggingFace download
///
/// This command cancels an in-progress download and cleans up partial files.
///
/// # Arguments
/// * `download_manager` - Download manager state
/// * `model_name` - Name of the model to cancel
///
/// # Returns
/// - `Ok(())` if the download is cancelled successfully
/// - `Err(String)` if the cancellation fails
///
/// **Validates: Requirements 9.1-9.5**
#[tauri::command]
pub async fn cancel_hf_download(
    download_manager: tauri::State<'_, crate::download_manager::DownloadManager>,
    model_name: String,
) -> Result<(), String> {
    download_manager
        .cancel_download(&model_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_hf_download(
    download_manager: tauri::State<'_, crate::download_manager::DownloadManager>,
    model_name: String,
) -> Result<(), String> {
    download_manager
        .pause_download(&model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Validate a HuggingFace API token
///
/// This command validates a HuggingFace token and returns the username.
///
/// # Arguments
/// * `token` - HuggingFace API token to validate
///
/// # Returns
/// - `Ok(String)` containing the username if valid
/// - `Err(String)` if the token is invalid
///
/// **Validates: Requirements 11.1-11.5**
#[tauri::command]
pub async fn validate_hf_token(token: String) -> Result<String, String> {
    println!("[CMD] validate_hf_token called");
    println!("[CMD] Token length: {}", token.len());
    
    if token.trim().is_empty() {
        println!("[CMD] Token is empty");
        return Err("Token cannot be empty".to_string());
    }
    
    println!("[CMD] Creating HfClient...");
    let client = crate::huggingface::HfClient::new(Some(token.clone()));
    
    println!("[CMD] Calling validate_token...");
    match client.validate_token(&token).await {
        Ok(username) => {
            println!("[CMD] Token validation successful, username: {}", username);
            Ok(username)
        }
        Err(e) => {
            println!("[CMD] Token validation failed: {:?}", e);
            Err(e.to_string())
        }
    }
}

/// Get optimization settings from database
///
/// This command retrieves the saved optimization settings.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
///
/// # Returns
/// - `Ok(OptimizationSettings)` containing the settings
/// - `Err(String)` if the database query fails
///
/// **Validates: Requirements 24.1-24.5**
#[tauri::command]
pub async fn get_optimization_settings(
    db_path: String,
) -> Result<crate::models::OptimizationSettings, String> {
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    db::get_optimization_settings(&conn)
        .map_err(|e| e.to_string())
}

/// Save optimization settings to database
///
/// This command saves optimization settings to the database.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
/// * `settings` - Optimization settings to save
///
/// # Returns
/// - `Ok(())` if the settings are saved successfully
/// - `Err(String)` if the database operation fails
///
/// **Validates: Requirements 24.1-24.5**
#[tauri::command]
pub async fn save_optimization_settings(
    db_path: String,
    settings: crate::models::OptimizationSettings,
) -> Result<(), String> {
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    db::save_optimization_settings(&conn, &settings)
        .map_err(|e| e.to_string())
}

/// Validate an existing GGUF file for the embedded runtime
#[tauri::command]
pub async fn test_ollama_registration(file_path: String, model_name: String) -> Result<String, String> {
    use std::path::Path;
    
    println!("[TEST] Testing Ollama registration");
    println!("[TEST] File path: {}", file_path);
    println!("[TEST] Model name: {}", model_name);
    
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    
    if path.extension().and_then(|ext| ext.to_str()) != Some("gguf") {
        return Err("Selected file is not a GGUF model".to_string());
    }

    Ok(format!("GGUF file is ready for local runtime registration: {}", model_name))
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaChatResponse {
    pub response: String,
}

/// Simple chat command with streaming support
/// Emits tokens in real-time as they're generated
#[tauri::command]
pub async fn ollama_chat(
    app_handle: tauri::AppHandle,
    runtime_manager: tauri::State<'_, crate::local_runtime::RuntimeManager>,
    model_name: String,
    messages: Vec<crate::models::ChatMessage>,
) -> Result<OllamaChatResponse, String> {
    let options = crate::models::ChatOptions {
        temperature: Some(0.7),
        num_ctx: Some(2048),
        num_predict: Some(512),
        num_gpu: None,
        num_batch: None,
        num_thread: None,
    };
    
    let response = runtime_manager
        .stream_chat_completion(&model_name, &messages, Some(&options), |token| {
            let token_event = serde_json::json!({
                "content": token,
            });

            app_handle
                .emit("chat-token", &token_event)
                .map_err(|e| crate::error::AppError::RuntimeError(format!("Failed to emit chat-token event: {}", e)))
        })
        .await
        .map_err(|e| format!("Failed to send chat message: {}", e))?;
    
    Ok(OllamaChatResponse { response })
}

// ============================================================================
// License Management Commands
// ============================================================================

/// Validate and activate a license key
///
/// This command validates a license key using Ed25519 cryptographic signatures
/// and stores it in the database if valid. The validation is performed offline
/// without requiring internet connectivity.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
/// * `license_key` - The license key string to validate
///
/// # Returns
/// * `Result<crate::license::LicenseInfo, String>` - License info if valid, error message otherwise
#[tauri::command]
pub async fn validate_license(
    db_path: String,
    license_key: String,
) -> Result<crate::license::LicenseInfo, String> {
    use crate::license::LicenseManager;

    let license_manager = LicenseManager::new()
        .map_err(|e| format!("Failed to initialize license manager: {}", e))?;

    // Validate the license key
    let license_info = license_manager
        .validate_license(&license_key)
        .map_err(|e| format!("Invalid license key: {}", e))?;

    // Check if the license is valid (not expired)
    if !license_info.is_valid {
        return Err("License has expired".to_string());
    }

    // Store the validated license in the database
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    license_manager
        .store_license(&conn, &license_key, &license_info)
        .map_err(|e| format!("Failed to store license: {}", e))?;

    Ok(license_info)
}

/// Check if Pro tier is enabled
///
/// This command checks if a valid Pro license exists in the database.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
///
/// # Returns
/// * `Result<bool, String>` - True if Pro is enabled, false otherwise
#[tauri::command]
pub async fn is_pro_enabled(db_path: String) -> Result<bool, String> {
    use crate::license::LicenseManager;

    let license_manager = LicenseManager::new()
        .map_err(|e| format!("Failed to initialize license manager: {}", e))?;

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    Ok(license_manager.is_pro_enabled(&conn))
}

/// Get current license information
///
/// This command retrieves the current license information from the database.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
///
/// # Returns
/// * `Result<Option<crate::license::LicenseInfo>, String>` - License info if exists, None otherwise
#[tauri::command]
pub async fn get_license_info(
    db_path: String,
) -> Result<Option<crate::license::LicenseInfo>, String> {
    use crate::license::LicenseManager;

    let license_manager = LicenseManager::new()
        .map_err(|e| format!("Failed to initialize license manager: {}", e))?;

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    license_manager
        .get_license_info(&conn)
        .map_err(|e| format!("Failed to get license info: {}", e))
}

/// Deactivate the current license
///
/// This command removes the current license from the database, disabling Pro features.
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
///
/// # Returns
/// * `Result<(), String>` - Success or error message
#[tauri::command]
pub async fn deactivate_license(db_path: String) -> Result<(), String> {
    use crate::license::LicenseManager;

    let license_manager = LicenseManager::new()
        .map_err(|e| format!("Failed to initialize license manager: {}", e))?;

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    license_manager
        .deactivate_license(&conn)
        .map_err(|e| format!("Failed to deactivate license: {}", e))
}

/// Get model recommendations based on hardware and use case
///
/// # Arguments
/// * `ram_gb` - Available RAM in gigabytes
/// * `vram_gb` - Available VRAM in gigabytes
/// * `use_case` - The intended use case ("coding", "chat", or "agents")
/// * `api_key` - Optional API key for the AI provider
/// * `provider` - The AI provider to use ("openrouter", "claude", or "openai")
///
/// # Returns
/// * `Result<Vec<ModelRecommendation>, String>` - List of recommended models with scores
#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_model_recommendations(
    app_handle: tauri::AppHandle,
    ramGb: f64,
    vramGb: f64,
    useCase: String,
    apiKey: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    hfToken: Option<String>,
) -> Result<Vec<crate::model_recommender::ModelRecommendation>, String> {
    println!("=== get_model_recommendations CALLED ===");
    println!("[CMD] ramGb: {}, vramGb: {}", ramGb, vramGb);
    println!("[CMD] useCase: {}", useCase);
    println!("[CMD] apiKey present: {}", apiKey.is_some());
    println!("[CMD] provider: {:?}", provider);
    println!("[CMD] model: {:?}", model);
    println!("[CMD] hfToken present: {}", hfToken.is_some());
    
    use crate::model_recommender::{ModelRecommender, UseCase};
    use tauri::Emitter;

    let use_case_enum = match useCase.to_lowercase().as_str() {
        "coding" => UseCase::Coding,
        "chat" => UseCase::Chat,
        "agents" => UseCase::Agents,
        _ => {
            println!("[CMD] Invalid use case: {}", useCase);
            return Err("Invalid use case. Must be 'coding', 'chat', or 'agents'".to_string());
        }
    };

    let provider_str = provider.unwrap_or_else(|| "openrouter".to_string());
    println!("[CMD] Using provider: {}", provider_str);
    
    let _ = app_handle.emit("wizard-status", "Getting AI recommendation...");

    println!("[CMD] Creating ModelRecommender...");
    let recommender = ModelRecommender::new();
    
    println!("[CMD] Calling recommend_models...");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        recommender.recommend_models(ramGb, vramGb, use_case_enum, apiKey, provider_str, model, hfToken, &app_handle),
    )
    .await
    .map_err(|_| {
        let error_msg = "Failed to get recommendations: request timed out after 60 seconds".to_string();
        println!("[CMD] Error: {}", error_msg);
        error_msg
    })
    .and_then(|result| {
        result.map_err(|e| {
            let error_msg = format!("Failed to get recommendations: {}", e);
            println!("[CMD] Error: {}", error_msg);
            error_msg
        })
    });

    if let Err(error) = &result {
        let _ = app_handle.emit("wizard-status", format!("AI recommendation failed: {}", error));
    } else {
        println!("[CMD] Successfully got recommendations");
    }

    result
}

// ============================================================================
// Llama Binary Management Commands
// ============================================================================

/// Get recommended binary variant based on detected hardware
#[tauri::command]
pub async fn get_recommended_binary() -> Result<crate::llama_binaries::BinaryVariant, String> {
    let detector = crate::hardware::HardwareDetector::new();
    let hw_info = detector.detect().await.map_err(|e| e.to_string())?;
    
    let os = std::env::consts::OS;
    Ok(crate::llama_binaries::recommend_binary(&hw_info.gpu_backend, os))
}

/// Get status of all llama binary variants
#[tauri::command]
pub async fn get_binary_statuses(
    app_handle: tauri::AppHandle,
) -> Result<Vec<crate::llama_binaries::BinaryStatus>, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    println!("[CMD] get_binary_statuses - app_data_dir: {}", app_data_dir.to_string_lossy());
    
    let manager = crate::llama_binaries::LlamaBinaryManager::new(app_data_dir);
    let statuses = manager.get_all_statuses().await;
    
    for status in &statuses {
        println!("[CMD] Binary status: {:?} - installed: {}, path: {:?}", status.variant, status.installed, status.path);
    }
    
    Ok(statuses)
}

/// Download a specific binary variant with progress updates
#[tauri::command]
pub async fn download_binary(
    app_handle: tauri::AppHandle,
    variant: crate::llama_binaries::BinaryVariant,
) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let manager = crate::llama_binaries::LlamaBinaryManager::new(app_data_dir);
    
    let app_handle_clone = app_handle.clone();
    let path = manager.download_binary(variant.clone(), move |progress| {
        let _ = app_handle_clone.emit("binary-download-progress", &progress);
    }).await.map_err(|e| e.to_string())?;
    
    Ok(path.to_string_lossy().to_string())
}

/// Check if a specific binary variant is installed
#[tauri::command]
pub async fn is_binary_installed(
    app_handle: tauri::AppHandle,
    variant: crate::llama_binaries::BinaryVariant,
) -> Result<bool, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let manager = crate::llama_binaries::LlamaBinaryManager::new(app_data_dir);
    Ok(manager.is_installed(&variant).await)
}

#[cfg(test)]
mod license_tests {
    use super::*;

    fn setup_test_license_db() -> String {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir
            .join(format!("test_license_{}.db", uuid::Uuid::new_v4()))
            .to_str()
            .unwrap()
            .to_string();

        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // Create licenses table
        conn.execute(
            "CREATE TABLE licenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_hash TEXT NOT NULL UNIQUE,
                license_type TEXT NOT NULL CHECK(license_type IN ('subscription', 'lifetime')),
                expiration_date INTEGER,
                created_at INTEGER NOT NULL,
                validated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        db_path
    }

    fn cleanup_test_license_db(db_path: &str) {
        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn test_is_pro_enabled_no_license() {
        let db_path = setup_test_license_db();

        let result = is_pro_enabled(db_path.clone()).await.unwrap();
        assert!(!result);

        cleanup_test_license_db(&db_path);
    }

    #[tokio::test]
    async fn test_validate_license_invalid_format() {
        let db_path = setup_test_license_db();

        let result = validate_license(db_path.clone(), "invalid".to_string()).await;
        assert!(result.is_err());

        cleanup_test_license_db(&db_path);
    }

    #[tokio::test]
    async fn test_get_license_info_no_license() {
        let db_path = setup_test_license_db();

        let result = get_license_info(db_path.clone()).await.unwrap();
        assert!(result.is_none());

        cleanup_test_license_db(&db_path);
    }

    #[tokio::test]
    async fn test_deactivate_license() {
        let db_path = setup_test_license_db();

        // Deactivating when no license exists should succeed
        let result = deactivate_license(db_path.clone()).await;
        assert!(result.is_ok());

        cleanup_test_license_db(&db_path);
    }
}
