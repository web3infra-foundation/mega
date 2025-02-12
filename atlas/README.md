## Atlas Module

Atlas is an LLM API crate that supports multiple LLM model platforms including ChatGPT, Claude, DeepSeek, etc.

### Support

Support ChatGPT, Claude, DeepSeek, Gemini, Gitee and 01.ai (https://www.lingyiwanwu.com / https://www.01.ai/)

**ChatGPT** `gpt-4o-mini` `gpt-4-turbo` `gpt-4` `gpt-3.5-turbo`

**Claude** `claude-3-5-sonnet-20240620` `claude-3-opus-20240229` `claude-3-sonnet-20240229` `claude-3-haiku-20240307`

**DeepSeek** `deepseek-chat` `deepseek-reasoner`

**Gemini** `models/chat-bison-001` `models/text-bison-001` `models/embedding-gecko-001` `models/gemini-1.0-pro-latest` `models/gemini-1.0-pro` `models/gemini-pro` `models/gemini-1.0-pro-001` `models/gemini-1.0-pro-vision-latest` `models/gemini-pro-vision` `models/gemini-1.5-pro-latest` `models/gemini-1.5-pro-001` `models/gemini-1.5-pro` `models/gemini-1.5-flash-latest` `models/gemini-1.5-flash-001` `models/gemini-1.5-flash` `models/embedding-001` `models/text-embedding-004` `models/aqa`

**Gitee** `YP3A1DT28TAJ` `H87ZZLSFILML` `KIXIB7TOZA1U`

**01.ai** `yi-large` `yi-medium` `yi-vision` `yi-medium-200k` `yi-spark` `vi-large-raq` `yi-large-turbo` `yi-large-fc`

### Example

```rust
// The required information is api_key, model name (of course atlas provides model name enumeration). 
// And if necessary, you can pass in api_base in some clients.
let api_key: &str = ...;
let api_base: Option<String> = ...;

// There are also enumerations of other supported models in OpenAIModels.
// For other LLM platforms, there are corresponding Models enumerations.
// Such as: ClaudeModels, DeepSeekModels etc
let model = OpenAIModels::GPT4O;

// The parameters are api key, model name and api base Option parameters.
let client = OpenAIClient::new(api_key, model, Some("https://..."));
// Or default api base. (Usually it is the official address)
let client = OpenAIClient::new(api_key, model, None);

// And, send message
let res = client.ask_model("Hello, I am an automated testing program. Please reply directly with \"Received\" without punctuation marks or unnecessary content.").await;

// LLM will reply "Received".
assert_eq!(res.unwrap(), "Received");

// It is also possible to construct multiple context messages.
let _context = crate::ChatMessage {
    messages: vec![
        (
            crate::ChatRole::User,
            "Response a '0' no matter what you receive".into(),
        ),
        (
            crate::ChatRole::Model,
            "Ok, I will response with a number 0.".into(),
        ),
        (crate::ChatRole::User, "who are you".into()),
    ],
};
let res = client.ask_model_with_context(_context).await;

// LLM will reply "0".
assert_eq!(res.unwrap(), "0");
```