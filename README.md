# leap-connect API client library for Rust
The leap-connect API client Rust library provides convenient access to the OpenAI API from Rust applications.

## Installation:
Cargo.toml
```toml
[dependencies]
leap-connect = "1.0.0"
```

## Usage
We recommend setting it as an environment variable. 
Here's an example of initializing the library with the API key loaded from an environment variable and creating a completion:

### Set TUPLELEAP_AI_API_KEY to environment variable
```bash
$ export TUPLELEAP_AI_API_KEY=sk-xxxxxxx
```

### Create client
```rust
let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());
```

### Create request
```rust
let req = ChatCompletionRequest::new(
    MISTRAL.to_string(),
    vec![chat_completion::ChatCompletionMessage {
        role: chat_completion::MessageRole::user,
        content: chat_completion::Content::Text(String::from("What is bitcoin?")),
        name: None,
    }],
);
```

### Send request
```rust
let result = client.chat_completion(req)?;
println!("Content: {:?}", result.choices[0].message.content);
```

### Set API_URL_V1 to environment variable
```bash
$ export API_URL_V1=https://api.tupleleap.ai/v1
```

## Example of chat completion
```rust
use leap_connect::v1::api::Client;
use leap_connect::v1::chat_completion::{self, ChatCompletionRequest};
use leap_connect::v1::common::MISTRAL;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("TUPLELEAP_AI_API_KEY").unwrap().to_string());

    let req = ChatCompletionRequest::new(
        GPT4_O.to_string(),
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
```
