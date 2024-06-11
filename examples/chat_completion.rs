use futures::StreamExt;
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
    println!("{}", env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());
    let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());
    let req = ChatCompletionRequest::new(
        MISTRAL.to_string(),
        vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(String::from("What is bitcoin?")),
            name: None,
        }],
    );

    let result_stream = client.chat_completion_stream(req).await?;
    let list: Vec<chat_completion::ChatChunkResponse> = result_stream.collect().await;
    for resp in list {
        for choice in resp.choices.iter() {
            let data = &choice.delta.content;
            if data.is_some() {
                print!("{}", data.clone().unwrap())
            }
        }
    }
    Ok(())
}

// OPENAI_API_KEY=xxxx cargo run --package openai-api-rs --example chat_completion
