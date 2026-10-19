// Copyright 2026 The Sashiko Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use super::{AiProvider, AiRequest, AiResponse, CacheStats, ModelRole, ProviderCapabilities};

/// Routes each request to the provider selected by its workflow stage.
pub struct RoutedProvider {
    main: Arc<dyn AiProvider>,
    validation: Arc<dyn AiProvider>,
}

impl RoutedProvider {
    pub fn new(main: Arc<dyn AiProvider>, validation: Arc<dyn AiProvider>) -> Self {
        Self { main, validation }
    }
}

#[async_trait]
impl AiProvider for RoutedProvider {
    async fn generate_content(&self, request: AiRequest) -> Result<AiResponse> {
        match request.model_role {
            ModelRole::Main => self.main.generate_content(request).await,
            ModelRole::Validation => self.validation.generate_content(request).await,
        }
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        self.main.get_capabilities()
    }

    fn cache_stats(&self) -> Option<CacheStats> {
        match (self.main.cache_stats(), self.validation.cache_stats()) {
            (None, None) => None,
            (main, validation) => {
                let main = main.unwrap_or_default();
                let validation = validation.unwrap_or_default();
                Some(CacheStats {
                    hits_this_session: main.hits_this_session + validation.hits_this_session,
                    hits_prev_session: main.hits_prev_session + validation.hits_prev_session,
                    tokens_saved_this_session: main.tokens_saved_this_session
                        + validation.tokens_saved_this_session,
                    tokens_saved_prev_session: main.tokens_saved_prev_session
                        + validation.tokens_saved_prev_session,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct NamedProvider(&'static str);

    #[async_trait]
    impl AiProvider for NamedProvider {
        async fn generate_content(&self, _request: AiRequest) -> Result<AiResponse> {
            Ok(AiResponse {
                content: Some(self.0.to_string()),
                thought: None,
                thought_signature: None,
                tool_calls: None,
                usage: None,
                truncated: false,
            })
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                model_name: self.0.to_string(),
                context_window_size: 1000,
            }
        }
    }

    #[tokio::test]
    async fn routes_validation_requests_and_defaults_old_requests_to_main() {
        let router = RoutedProvider::new(
            Arc::new(NamedProvider("main")),
            Arc::new(NamedProvider("validation")),
        );
        let mut request: AiRequest = serde_json::from_value(json!({ "messages": [] })).unwrap();
        assert_eq!(request.model_role, ModelRole::Main);
        assert_eq!(
            router
                .generate_content(request.clone())
                .await
                .unwrap()
                .content
                .as_deref(),
            Some("main")
        );

        request.model_role = ModelRole::Validation;
        let wire = serde_json::to_value(&request).unwrap();
        assert_eq!(wire["model_role"], "validation");
        let forwarded: AiRequest = serde_json::from_value(wire).unwrap();
        assert_eq!(
            router
                .generate_content(forwarded)
                .await
                .unwrap()
                .content
                .as_deref(),
            Some("validation")
        );
    }
}
