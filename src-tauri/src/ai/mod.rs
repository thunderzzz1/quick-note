pub struct AiClient {
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    api_key: String,
}

impl AiClient {
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        Self {
            base_url,
            model,
            api_key,
        }
    }
}
