mod rest_gemini_client;

use rest_gemini_client::AiClient;

#[tokio::main]
async fn main() {
    // get gemini api key from env variables
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");

    // create a new AiClient instance
    let client = AiClient::new(api_key);
    client.test_basic_connectivity().await.unwrap();
    client.call_llm_api("Hello, world!").await.unwrap();
}
