use leap_connect::v1::api::Client;
use leap_connect::v1::chat_completion::{self, ChatCompletionRequest};
use leap_connect::v1::common::MISTRAL;
use std::env;

/*
Add the following in settings.json file to run in vscode env
 "rust-analyzer.runnables.extraEnv": {
       "RUST_LOG": "debug",
       "TUPLELEAP_AI_API_KEY": "sk-xxxxxxx",
   },
   "rust-analyzer.cargo.extraEnv": {
       "RUST_LOG": "debug",
       "TUPLELEAP_AI_API_KEY": "sk-xxxxxxx",
   },
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==starting");
    println!(
        "=={}",
        env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string()
    );
    let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());
    println!("==client created");
    let req = ChatCompletionRequest::new(
        MISTRAL.to_string(),
        vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(String::from("What is bitcoin?")),
            name: None,
        }],
    );

    let result = client.chat_completion(req).await?;
    println!("==Content: {:?}", result.choices[0].message.content);
    println!("==Response Headers: {:?}", result.headers);

    Ok(())
}

// OPENAI_API_KEY=xxxx cargo run --package openai-api-rs --example chat_completion
