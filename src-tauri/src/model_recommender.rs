use crate::error::AppError;
use crate::huggingface::HfClient;
use tauri::Emitter;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model_id: String,
    pub model_name: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: Option<String>,
    pub suitability_score: f64,
    pub estimated_memory_gb: f64,
    pub estimated_tokens_per_sec: f64,
    pub quality_rating: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UseCase {
    Coding,
    Chat,
    Agents,
}

pub struct ModelRecommender;

const DEFAULT_OPENROUTER_FREE_MODEL: &str = "google/gemma-4-26b-a4b-it:free";

#[derive(Debug, Clone)]
struct VerifiedRepoCandidate {
    repo_id: String,
    filename: String,
    quantization: Option<String>,
    estimated_memory_gb: f64,
    downloads: i64,
    likes: i64,
    description: String,
}

impl ModelRecommender {
    pub fn new() -> Self {
        Self
    }

    pub async fn recommend_models(
        &self,
        available_ram_gb: f64,
        available_vram_gb: f64,
        use_case: UseCase,
        api_key: Option<String>,
        provider: String,
        model: Option<String>,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let provider = provider.to_lowercase();
        let api_key = clean_api_key(api_key).or_else(|| env_api_key_for_provider(&provider));

        if let Some(api_key) = api_key {
            let _ = app_handle.emit(
                "wizard-status",
                if provider == "openrouter" {
                    "Getting OpenRouter recommendation..."
                } else {
                    "Getting AI recommendation..."
                },
            );

            return match self
                .get_ai_recommendations(
                    available_ram_gb,
                    available_vram_gb,
                    &use_case,
                    api_key,
                    provider,
                    model,
                    hf_token,
                    app_handle,
                )
                .await
            {
                Ok(recommendations) => Ok(recommendations),
                Err(error) => {
                    println!(
                        "[ModelRecommender] AI recommendations failed; falling back to local recommendations: {}",
                        error
                    );
                    let _ = app_handle.emit(
                        "wizard-status",
                        "AI recommendation unavailable. Falling back to local model options...",
                    );
                    self.get_fallback_recommendations(
                        available_ram_gb,
                        available_vram_gb,
                        use_case,
                    )
                }
            };
        }

        let _ = app_handle.emit(
            "wizard-status",
            "No AI key configured. Finding public local models...",
        );

        let fallback = self.get_fallback_recommendations(
            available_ram_gb,
            available_vram_gb,
            use_case.clone(),
        )?;
        Ok(fallback)
    }

    async fn get_ai_recommendations(
        &self,
        available_ram_gb: f64,
        available_vram_gb: f64,
        use_case: &UseCase,
        api_key: String,
        provider: String,
        model: Option<String>,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let use_case_str = match use_case {
            UseCase::Coding => "code generation, refactoring, and technical tasks",
            UseCase::Chat => "conversational AI and general-purpose interactions",
            UseCase::Agents => "tool-calling and autonomous task execution",
        };
        let available_memory_gb = available_ram_gb.max(available_vram_gb);

        let prompt = format!(
            "You are an expert in local LLM deployment. Recommend exactly 5 strong local models for the following system.\n\n\
            - Use case: {}\n\
            - Available RAM: {:.1} GB\n\
            - Available VRAM: {:.1} GB\n\
            - Practical memory budget: {:.1} GB\n\n\
            Requirements:\n\
            - Recommend realistic GGUF-friendly local models that are commonly usable with Ollama or llama.cpp.\n\
            - Favor models that actually fit or nearly fit the available memory budget.\n\
            - Avoid duplicates, tiny toy models, and clearly impractical giant models.\n\
            - Prefer the strongest practical recommendations for the stated use case.\n\
            - Assume balanced quantizations such as Q4_K_M or Q5_K_M when estimating memory.\n\
            - Include a brief reason tailored to the use case and hardware.\n\n\
            Respond ONLY with a JSON array of 5 models in this exact format:\n\
            [{{\n  \
              \"model_id\": \"qwen2.5-coder:7b\",\n  \
              \"model_name\": \"Qwen 2.5 Coder 7B\",\n  \
              \"estimated_memory_gb\": 4.0,\n  \
              \"estimated_tokens_per_sec\": 22.0,\n  \
              \"quality_rating\": 0.85,\n  \
              \"suitability_score\": 0.91,\n  \
              \"description\": \"Fast and efficient for general tasks\"\n\
            }}]\n\n\
            Return only valid JSON with double-quoted keys and no markdown fences.",
            use_case_str, available_ram_gb, available_vram_gb, available_memory_gb
        );

        match provider.as_str() {
            "openrouter" => self.call_openrouter(&api_key, &prompt, model, available_memory_gb, use_case, hf_token, app_handle).await,
            "claude" => self.call_claude(&api_key, &prompt, model, available_memory_gb, use_case, hf_token, app_handle).await,
            "openai" => self.call_openai(&api_key, &prompt, model, available_memory_gb, use_case, hf_token, app_handle).await,
            _ => Err(AppError::HfError(format!("Unknown provider: {}", provider))),
        }
    }

    async fn call_openrouter(
        &self,
        api_key: &str,
        prompt: &str,
        model: Option<String>,
        available_memory_gb: f64,
        use_case: &UseCase,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let model_id = model.unwrap_or_else(|| DEFAULT_OPENROUTER_FREE_MODEL.to_string());
        println!("[ModelRecommender] OpenRouter using model: {}", model_id);

        let payload = serde_json::json!({
            "model": model_id,
            "max_tokens": 600,
            "stream": false,
            "messages": [
                {"role": "system", "content": "You recommend practical local GGUF models and respond with strict JSON only."},
                {"role": "user", "content": prompt}
            ]
        });

        let mut last_error: Option<AppError> = None;
        let mut body_text = None;

        for attempt in 1..=2 {
            println!("[ModelRecommender] Sending OpenRouter request to /chat/completions (attempt {})", attempt);

            let api_key = api_key.to_string();
            let payload = payload.clone();

            let response = tokio::time::timeout(
                Duration::from_secs(35),
                tokio::task::spawn_blocking(move || perform_openrouter_request(&api_key, &payload)),
            )
            .await
            .map_err(|_| AppError::HfError("OpenRouter request timed out after 35 seconds".to_string()))?
            .map_err(|e| AppError::HfError(format!("OpenRouter task failed: {}", e)))?;

            match response {
                Ok(text) => {
                    body_text = Some(text);
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(350 * attempt as u64)).await;
                    }
                }
            }
        }

        let body_text = body_text.ok_or_else(|| {
            last_error.unwrap_or_else(|| AppError::HfError("OpenRouter request failed without a response body".to_string()))
        })?;

        println!("[ModelRecommender] OpenRouter response length: {} bytes", body_text.len());

        let response_json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| AppError::HfError(format!("Failed to parse OpenRouter JSON: {} | Body: {}", e, &body_text[..body_text.len().min(500)])))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .map(|value| value.to_string())
            .or_else(|| {
                response_json["choices"][0]["message"]["content"]
                    .as_array()
                    .and_then(|items| items.first())
                    .and_then(|item| item["text"].as_str())
                    .map(|value| value.to_string())
            })
            .ok_or_else(|| AppError::HfError(format!("No content in OpenRouter response: {}", &body_text[..body_text.len().min(500)])))?;

        let parsed = self.parse_recommendations(&content, available_memory_gb, use_case)?;
        self.verify_recommendations(
            parsed,
            available_memory_gb,
            use_case,
            hf_token,
            app_handle,
        ).await
    }

    async fn call_claude(
        &self,
        api_key: &str,
        prompt: &str,
        model: Option<String>,
        available_memory_gb: f64,
        use_case: &UseCase,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let model_id = model.unwrap_or_else(|| "claude-sonnet-4-6".to_string());
        println!("[ModelRecommender] Claude using model: {}", model_id);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::HfError(format!("Failed to build HTTP client: {}", e)))?;

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model_id,
                "max_tokens": 2048,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await
            .map_err(|e| AppError::HfError(format!("Claude request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::HfError(format!("Claude API error ({}): {}", status, error_text)));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| AppError::HfError(format!("Failed to read Claude response body: {}", e)))?;

        let response_json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| AppError::HfError(format!("Failed to parse Claude JSON: {} | Body: {}", e, &body_text[..body_text.len().min(500)])))?;

        let content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AppError::HfError(format!("No content in Claude response: {}", &body_text[..body_text.len().min(500)])))?;

        let parsed = self.parse_recommendations(content, available_memory_gb, use_case)?;
        self.verify_recommendations(
            parsed,
            available_memory_gb,
            use_case,
            hf_token,
            app_handle,
        ).await
    }

    async fn call_openai(
        &self,
        api_key: &str,
        prompt: &str,
        model: Option<String>,
        available_memory_gb: f64,
        use_case: &UseCase,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let model_id = model.unwrap_or_else(|| "gpt-5.4".to_string());
        println!("[ModelRecommender] OpenAI using model: {}", model_id);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::HfError(format!("Failed to build HTTP client: {}", e)))?;

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model_id,
                "messages": [
                    {"role": "system", "content": "You recommend practical local GGUF models and respond with strict JSON only."},
                    {"role": "user", "content": prompt}
                ]
            }))
            .send()
            .await
            .map_err(|e| AppError::HfError(format!("OpenAI request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::HfError(format!("OpenAI API error ({}): {}", status, error_text)));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| AppError::HfError(format!("Failed to read OpenAI response body: {}", e)))?;

        let response_json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| AppError::HfError(format!("Failed to parse OpenAI JSON: {} | Body: {}", e, &body_text[..body_text.len().min(500)])))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .map(|value| value.to_string())
            .or_else(|| {
                response_json["choices"][0]["message"]["content"]
                    .as_array()
                    .and_then(|items| items.first())
                    .and_then(|item| item["text"].as_str())
                    .map(|value| value.to_string())
            })
            .ok_or_else(|| AppError::HfError(format!("No content in OpenAI response: {}", &body_text[..body_text.len().min(500)])))?;

        let parsed = self.parse_recommendations(&content, available_memory_gb, use_case)?;
        self.verify_recommendations(
            parsed,
            available_memory_gb,
            use_case,
            hf_token,
            app_handle,
        ).await
    }

    fn parse_recommendations(
        &self,
        content: &str,
        available_memory_gb: f64,
        use_case: &UseCase,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        // Extract JSON from markdown code blocks if present
        let json_str = if content.contains("```json") {
            content
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
                .trim()
        } else if content.contains("```") {
            content
                .split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
                .trim()
        } else {
            content.trim()
        };

        let models_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| AppError::HfError(format!("Failed to parse model recommendations: {}", e)))?;
        let models = if let Some(array) = models_value.as_array() {
            array.clone()
        } else if let Some(array) = models_value.get("models").and_then(|value| value.as_array()) {
            array.clone()
        } else {
            return Err(AppError::HfError("Model recommendations were not returned as an array".to_string()));
        };

        let mut recommendations = Vec::new();
        let use_case_bonus = match use_case {
            UseCase::Coding => [("coder", 0.1), ("code", 0.08), ("deepseek", 0.04), ("qwen", 0.03)].as_slice(),
            UseCase::Chat => [("instruct", 0.08), ("chat", 0.08), ("gemma", 0.03), ("llama", 0.03)].as_slice(),
            UseCase::Agents => [("tool", 0.08), ("function", 0.08), ("qwen", 0.04), ("llama", 0.03)].as_slice(),
        };

        for model in models {
            let mem_req = model["estimated_memory_gb"].as_f64().unwrap_or(4.0);
            let quality = model["quality_rating"].as_f64().unwrap_or(0.7);
            let base_name = format!(
                "{} {}",
                model["model_id"].as_str().unwrap_or(""),
                model["model_name"].as_str().unwrap_or("")
            )
            .to_lowercase();
            let hardware_fit = if mem_req <= available_memory_gb {
                1.0
            } else {
                (available_memory_gb / mem_req).clamp(0.0, 1.0)
            };
            let keyword_bonus = use_case_bonus
                .iter()
                .filter(|(keyword, _)| base_name.contains(*keyword))
                .map(|(_, bonus)| bonus)
                .sum::<f64>()
                .min(0.12);
            let estimated_tokens_per_sec = model["estimated_tokens_per_sec"]
                .as_f64()
                .unwrap_or_else(|| (32.0 / mem_req.max(1.0)).clamp(6.0, 40.0));
            let model_suitability = model["suitability_score"].as_f64().unwrap_or(0.0);
            let suitability = (quality * 0.45 + hardware_fit * 0.45 + keyword_bonus + model_suitability * 0.1).clamp(0.0, 1.0);

            recommendations.push(ModelRecommendation {
                model_id: model["model_id"].as_str().unwrap_or("unknown").to_string(),
                model_name: model["model_name"].as_str().unwrap_or("Unknown").to_string(),
                repo_id: String::new(),
                filename: String::new(),
                quantization: None,
                suitability_score: suitability,
                estimated_memory_gb: mem_req,
                estimated_tokens_per_sec,
                quality_rating: quality,
                description: model["description"].as_str().unwrap_or("").to_string(),
            });
        }

        recommendations.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        recommendations.dedup_by(|a, b| a.model_id == b.model_id);
        recommendations.truncate(5);

        if recommendations.is_empty() {
            return Err(AppError::HfError("No usable recommendations were returned".to_string()));
        }

        Ok(recommendations)
    }

    async fn verify_recommendations(
        &self,
        recommendations: Vec<ModelRecommendation>,
        available_memory_gb: f64,
        use_case: &UseCase,
        hf_token: Option<String>,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let _ = app_handle.emit("wizard-status", "Looking for models on Hugging Face...");
        let hf = HfClient::new(hf_token);
        let mut verified = Vec::new();

        for recommendation in recommendations {
            if let Some(repo) = self
                .find_verified_repo(&hf, &recommendation, available_memory_gb, use_case)
                .await?
            {
                let quality_boost =
                    ((repo.likes as f64 / 500.0).min(0.08) + (repo.downloads as f64 / 50_000.0).min(0.08)).min(0.12);
                let hardware_fit = if repo.estimated_memory_gb <= available_memory_gb {
                    1.0
                } else {
                    (available_memory_gb / repo.estimated_memory_gb).clamp(0.0, 1.0)
                };

                verified.push(ModelRecommendation {
                    model_id: recommendation.model_id,
                    model_name: recommendation.model_name,
                    repo_id: repo.repo_id,
                    filename: repo.filename,
                    quantization: repo.quantization,
                    suitability_score: (recommendation.suitability_score * 0.72 + hardware_fit * 0.18 + quality_boost)
                        .clamp(0.0, 1.0),
                    estimated_memory_gb: repo.estimated_memory_gb,
                    estimated_tokens_per_sec: recommendation.estimated_tokens_per_sec,
                    quality_rating: recommendation.quality_rating,
                    description: if recommendation.description.is_empty() {
                        repo.description
                    } else {
                        recommendation.description
                    },
                });
            }
        }

        verified.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        verified.dedup_by(|a, b| a.repo_id == b.repo_id);
        verified.truncate(5);

        if verified.is_empty() {
            return Err(AppError::HfError(
                "No verified open-source GGUF models were found for these recommendations".to_string(),
            ));
        }

        Ok(verified)
    }

    async fn find_verified_repo(
        &self,
        hf: &HfClient,
        recommendation: &ModelRecommendation,
        available_memory_gb: f64,
        use_case: &UseCase,
    ) -> Result<Option<VerifiedRepoCandidate>, AppError> {
        let queries = self.repo_queries(recommendation);
        let mut best: Option<(VerifiedRepoCandidate, f64)> = None;

        for query in queries {
            let repos = hf.search_models(&query, 0).await?;
            for repo in repos.into_iter().take(6) {
                let details = hf.get_model_details(&repo.id).await?;
                if !self.is_open_source_llm_repo(&repo.id, &details, use_case) {
                    continue;
                }

                let files = hf.get_model_files(&repo.id).await?;
                let Some(file) = self.select_best_file(&files, available_memory_gb) else {
                    continue;
                };

                let repo_text = format!(
                    "{} {}",
                    repo.id.to_lowercase(),
                    details.description.clone().unwrap_or_default().to_lowercase()
                );
                let name_match_bonus = self.query_match_bonus(&repo_text, recommendation);
                let hardware_fit = if file.estimated_ram_gb <= available_memory_gb {
                    1.0
                } else {
                    (available_memory_gb / file.estimated_ram_gb).clamp(0.0, 1.0)
                };
                let score = hardware_fit * 0.45
                    + recommendation.suitability_score * 0.25
                    + name_match_bonus * 0.15
                    + (repo.likes as f64 / 200.0).min(0.08)
                    + (repo.downloads as f64 / 20_000.0).min(0.07);

                let candidate = VerifiedRepoCandidate {
                    repo_id: repo.id.clone(),
                    filename: file.filename.clone(),
                    quantization: file.quantization.clone(),
                    estimated_memory_gb: file.estimated_ram_gb,
                    downloads: repo.downloads,
                    likes: repo.likes,
                    description: details.description.unwrap_or_default(),
                };

                match &best {
                    Some((_, best_score)) if *best_score >= score => {}
                    _ => best = Some((candidate, score)),
                }
            }

            if best.is_some() {
                break;
            }
        }

        Ok(best.map(|(candidate, _)| candidate))
    }

    fn repo_queries(&self, recommendation: &ModelRecommendation) -> Vec<String> {
        let mut queries = Vec::new();
        let base_id = recommendation.model_id.replace(':', " ").replace('-', " ");
        let base_name = recommendation.model_name.replace('-', " ");

        queries.push(format!("{} gguf", base_id));
        queries.push(format!("{} gguf instruct", base_name));

        let normalized = format!("{} {}", base_id, base_name).to_lowercase();
        if normalized.contains("qwen") {
            queries.push("qwen gguf instruct".to_string());
        }
        if normalized.contains("llama") {
            queries.push("llama gguf instruct".to_string());
        }
        if normalized.contains("mistral") {
            queries.push("mistral gguf instruct".to_string());
        }
        if normalized.contains("gemma") {
            queries.push("gemma gguf instruct".to_string());
        }

        queries.dedup();
        queries
    }

    fn is_open_source_llm_repo(
        &self,
        repo_id: &str,
        details: &crate::models::HfModelDetails,
        _use_case: &UseCase,
    ) -> bool {
        let haystack = format!(
            "{} {} {} {} {}",
            repo_id.to_lowercase(),
            details.pipeline_tag.clone().unwrap_or_default().to_lowercase(),
            details.license.clone().unwrap_or_default().to_lowercase(),
            details.description.clone().unwrap_or_default().to_lowercase(),
            details.tags.join(" ").to_lowercase()
        );

        let blocked_terms = [
            "bge", "embedding", "embeddings", "rerank", "reranker", "e5", "gte", "stella", "clip",
            "whisper", "asr", "text-to-image", "diffusion", "vision-language", "vision", "visual",
            "multimodal", "image-text-to-text", "multimodal-embedding",
            "reward-model", "classifier", "classification", "sentence-similarity", "feature-extraction",
        ];

        if blocked_terms.iter().any(|term| haystack.contains(term)) {
            return false;
        }

        let license = details.license.clone().unwrap_or_default().to_lowercase();
        if !license.is_empty() && (license.contains("proprietary") || license.contains("unknown")) {
            return false;
        }

        let llm_terms = ["text-generation", "causal-lm", "instruct", "chat", "assistant", "llm", "language model", "coder"];
        llm_terms.iter().any(|term| haystack.contains(term))
    }

    fn select_best_file(
        &self,
        files: &[crate::models::HfModelFile],
        available_memory_gb: f64,
    ) -> Option<crate::models::HfModelFile> {
        let preferred = ["Q4_K_M", "Q5_K_M", "Q4_K_S", "Q6_K", "Q8_0", "Q3_K_M", "Q3_K_S", "Q2_K"];

        preferred
            .iter()
            .find_map(|wanted| {
                files.iter().find(|file| {
                    file.quantization.as_deref() == Some(*wanted)
                        && file.estimated_ram_gb <= available_memory_gb * 1.15
                })
            })
            .cloned()
            .or_else(|| {
                files.iter()
                    .filter(|file| file.estimated_ram_gb <= available_memory_gb * 1.15)
                    .min_by(|a, b| {
                        let a_delta = (available_memory_gb - a.estimated_ram_gb).abs();
                        let b_delta = (available_memory_gb - b.estimated_ram_gb).abs();
                        a_delta.partial_cmp(&b_delta).unwrap()
                    })
                    .cloned()
            })
    }

    fn query_match_bonus(&self, repo_text: &str, recommendation: &ModelRecommendation) -> f64 {
        let mut bonus: f64 = 0.0;
        let needles = [
            recommendation.model_id.to_lowercase(),
            recommendation.model_name.to_lowercase(),
        ];

        for needle in needles {
            if needle.is_empty() {
                continue;
            }
            if repo_text.contains(&needle) {
                bonus += 0.12;
                continue;
            }
            for token in needle.split(|ch: char| !ch.is_ascii_alphanumeric()) {
                if token.len() >= 4 && repo_text.contains(token) {
                    bonus += 0.03;
                }
            }
        }

        bonus.min(0.2)
    }

    fn get_fallback_recommendations(
        &self,
        available_ram_gb: f64,
        available_vram_gb: f64,
        use_case: UseCase,
    ) -> Result<Vec<ModelRecommendation>, AppError> {
        let mut recommendations = Vec::new();

        let models = match use_case {
            UseCase::Coding => vec![
                ("deepseek-coder:6.7b", "DeepSeek Coder 6.7B", 8.0, 0.8, "Specialized for code generation"),
                ("codellama:7b", "Code Llama 7B", 8.0, 0.75, "Meta's code-focused model"),
                ("qwen2.5-coder:7b", "Qwen 2.5 Coder 7B", 8.0, 0.85, "Excellent for coding tasks"),
                ("llama3.2:3b", "Llama 3.2 3B", 4.0, 0.7, "Fast and efficient"),
                ("phi3:3.8b", "Phi-3 3.8B", 4.0, 0.65, "Compact and capable"),
            ],
            UseCase::Chat => vec![
                ("llama3.2:3b", "Llama 3.2 3B", 4.0, 0.85, "Great for conversations"),
                ("mistral:7b", "Mistral 7B", 8.0, 0.8, "Balanced performance"),
                ("phi3:3.8b", "Phi-3 3.8B", 4.0, 0.75, "Efficient chat model"),
                ("qwen2.5:7b", "Qwen 2.5 7B", 8.0, 0.82, "Strong general purpose"),
                ("gemma2:2b", "Gemma 2 2B", 3.0, 0.7, "Very fast responses"),
            ],
            UseCase::Agents => vec![
                ("llama3.1:8b", "Llama 3.1 8B", 10.0, 0.85, "Excellent tool calling"),
                ("mistral-nemo:12b", "Mistral Nemo 12B", 14.0, 0.8, "Advanced reasoning"),
                ("qwen2.5:7b", "Qwen 2.5 7B", 8.0, 0.75, "Good for agents"),
                ("llama3.2:3b", "Llama 3.2 3B", 4.0, 0.7, "Lightweight agent"),
                ("phi3:3.8b", "Phi-3 3.8B", 4.0, 0.68, "Compact agent model"),
            ],
        };

        for (id, name, mem_req, quality, desc) in models {
            let hardware_fit = if mem_req <= available_ram_gb.max(available_vram_gb) {
                1.0
            } else {
                (available_ram_gb.max(available_vram_gb) / mem_req).min(1.0)
            };

            let suitability = hardware_fit * 0.6 + quality * 0.4;

            recommendations.push(ModelRecommendation {
                model_id: id.to_string(),
                model_name: name.to_string(),
                repo_id: String::new(),
                filename: String::new(),
                quantization: None,
                suitability_score: suitability,
                estimated_memory_gb: mem_req,
                estimated_tokens_per_sec: 20.0,
                quality_rating: quality,
                description: desc.to_string(),
            });
        }

        recommendations.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        Ok(recommendations.into_iter().take(5).collect())
    }
}

fn clean_api_key(api_key: Option<String>) -> Option<String> {
    api_key
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

fn env_api_key_for_provider(provider: &str) -> Option<String> {
    match provider {
        "openrouter" => env::var("OPENROUTER_API_KEY").ok(),
        "claude" => env::var("ANTHROPIC_API_KEY").ok(),
        "openai" => env::var("OPENAI_API_KEY").ok(),
        _ => None,
    }
    .and_then(|key| clean_api_key(Some(key)))
}

fn perform_openrouter_request(api_key: &str, payload: &serde_json::Value) -> Result<String, AppError> {
    println!("[ModelRecommender] Preparing OpenRouter HTTP request");

    let request = attohttpc::post("https://openrouter.ai/api/v1/chat/completions")
        .connect_timeout(Duration::from_secs(8))
        .proxy_settings(attohttpc::ProxySettings::builder().build())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("HTTP-Referer", "https://openllm.studio")
        .header("X-Title", "OpenLLM Studio")
        .json(payload)
        .map_err(|e| AppError::HfError(format!("Failed to serialize OpenRouter request: {}", e)))?;

    println!("[ModelRecommender] OpenRouter send() starting");
    let response = request
        .send()
        .map_err(|e| AppError::HfError(format!("OpenRouter request failed: {}", e)))?;
    println!("[ModelRecommender] OpenRouter send() finished with status {}", response.status());
    println!(
        "[ModelRecommender] OpenRouter headers: content-length={:?}, transfer-encoding={:?}, content-encoding={:?}",
        response.headers().get("content-length").and_then(|v| v.to_str().ok()),
        response.headers().get("transfer-encoding").and_then(|v| v.to_str().ok()),
        response.headers().get("content-encoding").and_then(|v| v.to_str().ok())
    );

    if !response.is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(AppError::HfError(format!("OpenRouter API error ({}): {}", status, error_text)));
    }

    println!("[ModelRecommender] OpenRouter body read starting");
    let (headers, mut reader) = {
        let (_status, headers, reader) = response.split();
        (headers, reader)
    };

    let body_bytes = if let Some(length) = headers
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
    {
        let mut bytes = vec![0u8; length];
        reader
            .read_exact(&mut bytes)
            .map_err(|e| AppError::HfError(format!("Failed to read OpenRouter response body: {}", e)))?;
        bytes
    } else {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 4096];

        loop {
            let read = reader
                .read(&mut chunk)
                .map_err(|e| AppError::HfError(format!("Failed to read OpenRouter response body: {}", e)))?;

            if read == 0 {
                break;
            }

            bytes.extend_from_slice(&chunk[..read]);

            if serde_json::from_slice::<serde_json::Value>(&bytes).is_ok() {
                println!("[ModelRecommender] OpenRouter JSON became complete before EOF");
                break;
            }
        }

        bytes
    };

    println!("[ModelRecommender] OpenRouter body read finished ({} bytes)", body_bytes.len());
    Ok(String::from_utf8_lossy(&body_bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[tokio::test]
    async fn openrouter_outer_timeout_is_enforced_on_slow_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            thread::sleep(Duration::from_millis(250));
            let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}";
            let _ = stream.write_all(response.as_bytes());
        });

        let result = tokio::time::timeout(
            Duration::from_millis(50),
            tokio::task::spawn_blocking(move || {
                attohttpc::post(format!("http://127.0.0.1:{port}"))
                    .connect_timeout(Duration::from_secs(1))
                    .json(&serde_json::json!({"ok": true}))
                    .unwrap()
                    .send()
            }),
        )
        .await;

        assert!(result.is_err(), "outer timeout should fire before the slow response completes");
        handle.join().unwrap();
    }
}
