use leap_connect::v1::api::Client;
use leap_connect::v1::chat_completion::{self, ChatCompletionRequest};
use leap_connect::v1::common::{MISTRAL};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());

    let  req = ChatCompletionRequest::new(
        MISTRAL.to_string(),
        vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(String::from("What is bitcoin?")),
            name: None,

        }],
    );

    let result = client.chat_completion(req)?;
    println!("Content: {:?}", result.choices[0].message.content);
    println!("Response Headers: {:?}", result.headers);

    Ok(())
}

// OPENAI_API_KEY=xxxx cargo run --package openai-api-rs --example chat_completion
