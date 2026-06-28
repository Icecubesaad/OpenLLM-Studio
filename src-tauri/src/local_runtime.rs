use crate::error::AppError;
use crate::model_store;
use crate::models::{ChatMessage, ChatOptions, LoadedModel, Model, OllamaVersion};
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const DEFAULT_RUNTIME_PORT: u16 = 12434;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct ActiveRuntime {
    model_name: String,
    model_size: i64,
    port: u16,
    process: Child,
}

pub struct RuntimeManager {
    model_root: PathBuf,
    runtime_root: PathBuf,
    client: reqwest::Client,
    state: Arc<Mutex<Option<ActiveRuntime>>>,
    active_chat_cancel: Arc<Mutex<Option<CancellationToken>>>,
}

impl RuntimeManager {
    pub fn new(model_root: PathBuf, runtime_root: PathBuf) -> Self {
        Self {
            model_root,
            runtime_root,
            client: reqwest::Client::new(),
            state: Arc::new(Mutex::new(None)),
            active_chat_cancel: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn health_check(&self) -> Result<OllamaVersion, AppError> {
        Ok(OllamaVersion {
            version: "embedded-llama.cpp".to_string(),
        })
    }

    pub async fn list_models(&self) -> Result<Vec<Model>, AppError> {
        model_store::list_installed_models(&self.model_root)
    }

    pub async fn get_loaded_model(&self) -> Result<Option<LoadedModel>, AppError> {
        let state = self.state.lock().await;
        Ok(state.as_ref().map(|runtime| LoadedModel {
            name: runtime.model_name.clone(),
            size: runtime.model_size,
        }))
    }

    pub async fn delete_model(&self, model_name: String) -> Result<(), AppError> {
        self.unload_if_active(&model_name).await?;
        model_store::delete_model(&self.model_root, &model_name)
    }

    pub async fn ensure_model_loaded(
        &self,
        model_name: &str,
        options: Option<&ChatOptions>,
    ) -> Result<String, AppError> {
        let manifest = model_store::get_manifest(&self.model_root, model_name)?
            .ok_or_else(|| AppError::RuntimeError(format!("Model '{}' is not installed", model_name)))?;
        let model_path = PathBuf::from(&manifest.file_path);

        {
            let mut state = self.state.lock().await;
            if let Some(active) = state.as_mut() {
                if active.model_name == model_name && active.process.try_wait()?.is_none() {
                    return Ok(format!("http://127.0.0.1:{}", active.port));
                }
            }
        }

        self.stop_runtime().await?;
        let port = DEFAULT_RUNTIME_PORT;
        let process = self.spawn_runtime_process(&model_path, options, port).await?;
        self.wait_for_runtime(port).await?;

        let mut state = self.state.lock().await;
        *state = Some(ActiveRuntime {
            model_name: manifest.name,
            model_size: manifest.size,
            port,
            process,
        });

        Ok(format!("http://127.0.0.1:{}", port))
    }

    pub async fn stream_chat_completion(
        &self,
        model_name: &str,
        messages: &[ChatMessage],
        options: Option<&ChatOptions>,
        mut on_token: impl FnMut(String) -> Result<(), AppError>,
    ) -> Result<String, AppError> {
        let cancel_token = CancellationToken::new();
        {
            let mut active_cancel = self.active_chat_cancel.lock().await;
            *active_cancel = Some(cancel_token.clone());
        }

        let base_url = self.ensure_model_loaded(model_name, options).await?;

        // Prepend a system prompt if one isn't already present
        let messages_with_system: Vec<ChatMessage>;
        let effective_messages: &[ChatMessage] = if messages.first().map(|m| m.role.as_str()) == Some("system") {
            messages
        } else {
            messages_with_system = std::iter::once(ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant. Format your responses using proper Markdown:\n- Use **bold** and *italic* for emphasis\n- Use # ## ### for headings, always preceded and followed by a blank line\n- Use fenced code blocks with a language tag on their own line, e.g.:\n  ```python\n  code here\n  ```\n- Use - or 1. for lists\n- Separate paragraphs with a blank line\n- Never run headings, code fences, or list items together on the same line as other text".to_string(),
            })
            .chain(messages.iter().cloned())
            .collect();
            &messages_with_system
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&serde_json::json!({
                "model": model_name,
                "messages": effective_messages,
                "stream": true,
                "temperature": 0.7,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::RuntimeError(format!(
                "Local runtime rejected the chat request: {} {}",
                status, body
            )));
        }

        let mut full_response = String::new();
        let mut pending = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            if cancel_token.is_cancelled() {
                let mut active_cancel = self.active_chat_cancel.lock().await;
                *active_cancel = None;
                return Ok(full_response);
            }

            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    let mut active_cancel = self.active_chat_cancel.lock().await;
                    *active_cancel = None;
                    if cancel_token.is_cancelled() {
                        return Ok(full_response);
                    }
                    return Err(error.into());
                }
            };
            pending.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = pending.find('\n') {
                let line = pending[..line_end].trim().to_string();
                pending = pending[line_end + 1..].to_string();

                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let payload = line.trim_start_matches("data: ").trim();
                if payload == "[DONE]" {
                    continue;
                }

                let parsed: OpenAiStreamChunk = serde_json::from_str(payload)?;
                if let Some(token) = parsed
                    .choices
                    .into_iter()
                    .find_map(|choice| choice.delta.and_then(|delta| delta.content))
                {
                    full_response.push_str(&token);
                    on_token(token)?;
                }
            }
        }

        let mut active_cancel = self.active_chat_cancel.lock().await;
        *active_cancel = None;
        Ok(full_response)
    }

    pub async fn stop_generation(&self) -> Result<(), AppError> {
        if let Some(token) = self.active_chat_cancel.lock().await.take() {
            token.cancel();
        }
        self.stop_runtime().await
    }

    pub async fn stop_runtime(&self) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        if let Some(active) = state.as_mut() {
            if active.process.try_wait()?.is_none() {
                let _ = active.process.kill().await;
            }
        }
        *state = None;
        Ok(())
    }

    async fn unload_if_active(&self, model_name: &str) -> Result<(), AppError> {
        let should_stop = {
            let state = self.state.lock().await;
            state
                .as_ref()
                .map(|runtime| runtime.model_name == model_name)
                .unwrap_or(false)
        };

        if should_stop {
            self.stop_runtime().await?;
        }

        Ok(())
    }

    async fn spawn_runtime_process(
        &self,
        model_path: &Path,
        options: Option<&ChatOptions>,
        port: u16,
    ) -> Result<Child, AppError> {
        let server_binary = self.resolve_server_binary()?;
        let num_ctx = options.and_then(|opts| opts.num_ctx).unwrap_or(8192);
        let num_gpu_layers = options.and_then(|opts| opts.num_gpu).unwrap_or(-1);
        let batch_size = options.and_then(|opts| opts.num_batch).unwrap_or(512);
        let num_threads = options
            .and_then(|opts| opts.num_thread)
            .unwrap_or_else(default_thread_count);

        let mut command = Command::new(server_binary);
        command
            .arg("-m")
            .arg(model_path)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("-c")
            .arg(num_ctx.to_string())
            .arg("-ngl")
            .arg(num_gpu_layers.to_string())
            .arg("-b")
            .arg(batch_size.to_string())
            .arg("-t")
            .arg(num_threads.to_string())
            .arg("--no-warmup")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|e| {
            AppError::RuntimeError(format!(
                "Failed to start embedded llama.cpp runtime. Expected binary near '{}': {}",
                self.runtime_root.display(),
                e
            ))
        })?;

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    eprintln!("[llama-server] {}", line);
                }
            });
        }

        Ok(child)
    }

    async fn wait_for_runtime(&self, port: u16) -> Result<(), AppError> {
        let url = format!("http://127.0.0.1:{}/v1/models", port);

        for _ in 0..180 {
            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => return Ok(()),
                _ => tokio::time::sleep(tokio::time::Duration::from_millis(500)).await,
            }
        }

        Err(AppError::RuntimeError(
            "Embedded llama.cpp runtime did not become ready in time".to_string(),
        ))
    }

    fn resolve_server_binary(&self) -> Result<PathBuf, AppError> {
        if let Ok(path) = std::env::var("OPENLLM_LLAMA_SERVER_PATH") {
            let binary = PathBuf::from(path);
            if binary.exists() {
                return Ok(binary);
            }
        }

        let binary_name = if cfg!(target_os = "windows") {
            "llama-server.exe"
        } else {
            "llama-server"
        };

        let candidates = if cfg!(target_os = "windows") {
            self.windows_binary_candidates(binary_name)
        } else {
            self.unix_binary_candidates(binary_name)
        };

        for candidate in &candidates {
            println!("[RuntimeManager] Checking binary candidate: {} -> exists: {}", candidate.display(), candidate.exists());
            if candidate.exists() {
                println!("[RuntimeManager] Found binary at: {}", candidate.display());
                return Ok(candidate.clone());
            }
        }

        Err(AppError::RuntimeError(format!(
            "Could not find a usable llama.cpp server binary. Checked: {}. Put your binaries under '{}'",
            candidates
                .iter()
                .map(|path| format!("'{}'", path.display()))
                .collect::<Vec<_>>()
                .join(", "),
            self.runtime_root.display()
        )))
    }

    fn windows_binary_candidates(&self, binary_name: &str) -> Vec<PathBuf> {
        let has_nvidia = Self::has_command("nvidia-smi");
        let has_amd = Self::has_windows_amd_gpu();
        let mut candidates = Vec::new();

        // ---- 1. Downloaded binaries directory (app_data_dir/llama-binaries/) ----
        // This is where LlamaBinaryManager downloads binaries to.
        // The model_root is app_data_dir/models, so go up to get app_data_dir.
        let app_data_dir = self.model_root.parent().map(PathBuf::from);
        if let Some(ref app_dir) = app_data_dir {
            let binaries_dir = app_dir.join("llama-binaries");
            if has_nvidia {
                candidates.push(binaries_dir.join("cuda-13.1").join(binary_name));
                candidates.push(binaries_dir.join("cuda-12.4").join(binary_name));
            }
            if has_amd {
                candidates.push(binaries_dir.join("vulkan").join(binary_name));
            }
            candidates.push(binaries_dir.join("cpu").join(binary_name));
            candidates.push(binaries_dir.join("metal").join(binary_name));
            // Fallback: flat path for previously-downloaded binaries
            candidates.push(binaries_dir.join(binary_name));
        }

        // ---- 2. Runtime root (resource_dir/llama/) ----
        if has_nvidia {
            candidates.push(self.runtime_root.join("windows").join("cuda-13.1").join(binary_name));
            candidates.push(self.runtime_root.join("windows").join("cuda-12.4").join(binary_name));
        }
        if has_amd {
            candidates.push(self.runtime_root.join("windows").join("vulkan").join(binary_name));
        }

        candidates.push(self.runtime_root.join("windows").join("cpu").join(binary_name));
        candidates.push(self.runtime_root.join(binary_name));
        candidates.push(self.runtime_root.join("cpu").join(binary_name));

        if has_nvidia {
            candidates.push(self.runtime_root.join("cuda-13.1").join(binary_name));
            candidates.push(self.runtime_root.join("cuda-12.4").join(binary_name));
        }
        if has_amd {
            candidates.push(self.runtime_root.join("vulkan").join(binary_name));
        }

        // ---- 3. Source resources (for dev mode) ----
        let source_resource_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("llama");

        if has_nvidia {
            candidates.push(source_resource_root.join("windows").join("cuda-13.1").join(binary_name));
            candidates.push(source_resource_root.join("windows").join("cuda-12.4").join(binary_name));
        }
        if has_amd {
            candidates.push(source_resource_root.join("windows").join("vulkan").join(binary_name));
        }
        candidates.push(source_resource_root.join("windows").join("cpu").join(binary_name));
        candidates.push(source_resource_root.join(binary_name));

        candidates
    }

    fn unix_binary_candidates(&self, binary_name: &str) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(app_dir) = self.model_root.parent() {
            let binaries_dir = app_dir.join("llama-binaries");
            let variant_dirs: &[&str] = if cfg!(target_os = "macos") {
                &["metal", "cpu"]
            } else {
                &["cuda-13.1", "cuda-12.4", "vulkan", "cpu"]
            };

            for variant_dir in variant_dirs {
                let root = binaries_dir.join(variant_dir);
                candidates.push(root.join(binary_name));
                candidates.extend(Self::find_named_files(&root, binary_name));
            }

            candidates.push(binaries_dir.join(binary_name));
        }

        if cfg!(target_os = "macos") {
            candidates.push(self.runtime_root.join("macos").join("metal").join(binary_name));
            candidates.push(self.runtime_root.join("macos").join("cpu").join(binary_name));
            candidates.push(self.runtime_root.join("metal").join(binary_name));
            candidates.push(self.runtime_root.join("cpu").join(binary_name));
        } else {
            candidates.push(self.runtime_root.join("linux").join("cuda-13.1").join(binary_name));
            candidates.push(self.runtime_root.join("linux").join("cuda-12.4").join(binary_name));
            candidates.push(self.runtime_root.join("linux").join("vulkan").join(binary_name));
            candidates.push(self.runtime_root.join("linux").join("cpu").join(binary_name));
            candidates.push(self.runtime_root.join("cuda-13.1").join(binary_name));
            candidates.push(self.runtime_root.join("cuda-12.4").join(binary_name));
            candidates.push(self.runtime_root.join("vulkan").join(binary_name));
            candidates.push(self.runtime_root.join("cpu").join(binary_name));
        }

        candidates.push(self.runtime_root.join(binary_name));

        let source_resource_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("llama");

        if cfg!(target_os = "macos") {
            candidates.push(source_resource_root.join("macos").join("metal").join(binary_name));
            candidates.push(source_resource_root.join("macos").join("cpu").join(binary_name));
            candidates.push(source_resource_root.join("metal").join(binary_name));
            candidates.push(source_resource_root.join("cpu").join(binary_name));
        } else {
            candidates.push(source_resource_root.join("linux").join("cuda-13.1").join(binary_name));
            candidates.push(source_resource_root.join("linux").join("cuda-12.4").join(binary_name));
            candidates.push(source_resource_root.join("linux").join("vulkan").join(binary_name));
            candidates.push(source_resource_root.join("linux").join("cpu").join(binary_name));
            candidates.push(source_resource_root.join("cuda-13.1").join(binary_name));
            candidates.push(source_resource_root.join("cuda-12.4").join(binary_name));
            candidates.push(source_resource_root.join("vulkan").join(binary_name));
            candidates.push(source_resource_root.join("cpu").join(binary_name));
        }

        candidates.push(source_resource_root.join(binary_name));
        candidates
    }
}

impl Drop for RuntimeManager {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_lock() {
            if let Some(active) = state.as_mut() {
                let _ = active.process.start_kill();
            }
            *state = None;
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: Option<OpenAiStreamDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
}

fn default_thread_count() -> i32 {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() as i32)
        .unwrap_or(4)
}

impl RuntimeManager {
    fn find_named_files(root: &Path, binary_name: &str) -> Vec<PathBuf> {
        let mut found = Vec::new();
        Self::collect_named_files(root, binary_name, &mut found);
        found
    }

    fn collect_named_files(root: &Path, binary_name: &str, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some(binary_name) && path.is_file() {
                found.push(path);
                continue;
            }
            if path.is_dir() {
                Self::collect_named_files(&path, binary_name, found);
            }
        }
    }

    fn has_command(command: &str) -> bool {
        StdCommand::new(command)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn has_windows_amd_gpu() -> bool {
        if !cfg!(target_os = "windows") {
            return false;
        }

        let output = StdCommand::new("wmic")
            .args(["path", "win32_VideoController", "get", "name"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
                text.contains("amd") || text.contains("radeon")
            }
            _ => false,
        }
    }
}
