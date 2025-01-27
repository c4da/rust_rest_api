use std::{error::Error, time::Duration, net::ToSocketAddrs};
use std::fmt;
use reqwest::{Client, ClientBuilder};
use serde_json::{json, Value};
use bevy::prelude::*;

pub const INIT_MESSAGE: &str = "\
enter a prompt to generate a response from the AI model
{
    \"command\": \"value\",
    \"parameters\": {
        \"size\": 1,
        \"color\": \"red\"
    }
}
Always respond with valid JSON in the exact format shown above. Here is the prompt:";

#[derive(Resource, Clone)]
pub struct AiClient {
    api_key: String,
    client: Client,
}

impl Default for AiClient {
    fn default() -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .pool_max_idle_per_host(0)
            .build()
            .expect("Failed to create client");
        Self {
            api_key: String::new(),
            client,
        }
    }
}

impl AiClient {
    pub fn new(api_key: String) -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .pool_max_idle_per_host(0)
            .build()
            .expect("Failed to create client");
        Self {
            api_key,
            client,
        }
    }

    pub async fn test_basic_connectivity(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let host = "generativelanguage.googleapis.com:443";
        println!("Testing basic DNS resolution for: {}", host);
        
        match host.to_socket_addrs() {
            Ok(addrs) => {
                for addr in addrs {
                    println!("Resolved address: {}", addr);
                }
                Ok(())
            },
            Err(e) => {
                println!("DNS resolution failed: {}", e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn call_llm_api(&self, prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        println!("Testing connectivity first...");
        if let Err(e) = self.test_basic_connectivity().await {
            println!("Connectivity test failed: {:?}", e);
            return Err(e);
        }
        println!("Connectivity test passed, proceeding with API call");
        println!("Calling LLM API with prompt: {}", prompt);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            self.api_key
        );

        let request_body = json!({
            "contents": [{
                "parts": [{
                    "text": format!("{}{}", INIT_MESSAGE, prompt)
                }],
                "role": "user"
            }],
            "generationConfig": {
                "temperature": 0.1,
                "topK": 1,
                "topP": 1
            }
        });

        match self.client
            .post(&url)
            .header("Host", "generativelanguage.googleapis.com")
            .json(&request_body)
            .send()
            .await
        {
            Ok(response) => {
                println!("Received response with status: {}", response.status());
                handle_response(response).await
            },
            Err(e) => {
                println!("Error sending request: {:?}", e);
                if let Some(status) = e.status() {
                    println!("HTTP status code: {}", status);
                }
                if let Some(url) = e.url() {
                    println!("Failed URL: {}", url);
                }
                Err(Box::new(e))
            }
        }
    }
}

async fn handle_response(res: reqwest::Response) -> Result<String, Box<dyn Error + Send + Sync>> {
    let status = res.status();
    if status.is_success() {
        let response_json: serde_json::Value = res.json().await?;
        println!("Response: {}", response_json);
        
        // Extract the text from Gemini's response format
        let raw_text = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| ApiError {
                status,
                message: "Failed to extract text from response".to_string(),
            })?;

        // Clean response
        let trimmed = raw_text.trim();
        let without_markers = trimmed.trim_start_matches("```json").trim_end_matches("```");
        let without_newlines = without_markers.replace("\n", "");
        let text = without_newlines.trim();

        // Parse the text as JSON to validate it's a proper JSON response
        let json_value: Value = serde_json::from_str(text)
            .map_err(|e| ApiError {
                status,
                message: format!("Invalid JSON in response: {}", e),
            })?;

        // Verify the JSON has the expected structure
        if !json_value.is_object() || !json_value["command"].is_string() {
            return Err(Box::new(ApiError {
                status,
                message: "Response JSON missing required fields".to_string(),
            }));
        }

        // Check parameters based on command type
        let command = json_value["command"].as_str().unwrap();
        match command {
            "greeting" => {
                if !json_value["parameters"].is_object() {
                    return Err(Box::new(ApiError {
                        status,
                        message: "Single parameters must be an object".to_string(),
                    }));
                }
            },
            "1" => {
                if !json_value["parameters"].is_object() {
                    return Err(Box::new(ApiError {
                        status,
                        message: "Single parameters must be an object".to_string(),
                    }));
                }
            },
            "2" => {
                if !json_value["parameters"].is_array() {
                    return Err(Box::new(ApiError {
                        status,
                        message: "Multiple parameters must be an array".to_string(),
                    }));
                }
            },
            _ => {
                return Err(Box::new(ApiError {
                    status,
                    message: format!("Unknown command: {}", command),
                }));
            }
        }

        Ok(text.to_string())
    } else {
        let error_body = res.text().await.unwrap_or_else(|_| "Error reading response".to_string());
        println!("Error: {} - {}", status, error_body);
        Err(Box::new(ApiError {
            status,
            message: error_body,
        }))
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub status: reqwest::StatusCode,
    pub message: String,
}

unsafe impl Send for ApiError {}
unsafe impl Sync for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API Error: {} - {}", self.status, self.message)
    }
}

impl Error for ApiError {}
