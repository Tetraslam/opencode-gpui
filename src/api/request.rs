use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use url::Url;

use crate::model::{MessageRecord, Session};

use super::{ClientInner, CreateSession, Error, Prompt};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionBody {
    #[serde(rename = "parentID", skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Serialize)]
struct UpdateSessionBody<'a> {
    title: &'a str,
}

#[derive(Serialize)]
struct PromptBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<crate::model::ModelRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    parts: [TextPartInput; 1],
}

#[derive(Serialize)]
struct TextPartInput {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

impl ClientInner {
    pub(super) async fn create_session(&self, options: CreateSession) -> Result<Session, Error> {
        let url = self.scoped_url(&["session"])?;
        let body = CreateSessionBody {
            parent_id: options.parent_id,
            title: options.title,
        };
        Self::send_json(self.request_url_method(Method::POST, url).json(&body)).await
    }

    pub(super) async fn rename_session(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<Session, Error> {
        let url = self.scoped_url(&["session", session_id])?;
        Self::send_json(
            self.request_url_method(Method::PATCH, url)
                .json(&UpdateSessionBody { title }),
        )
        .await
    }

    pub(super) async fn session_boolean(
        &self,
        method: Method,
        session_id: &str,
        action: Option<&str>,
    ) -> Result<bool, Error> {
        let mut segments = vec!["session", session_id];
        if let Some(action) = action {
            segments.push(action);
        }
        let url = self.scoped_url(&segments)?;
        Self::send_json(self.request_url_method(method, url)).await
    }

    pub(super) async fn children(&self, session_id: &str) -> Result<Vec<Session>, Error> {
        let url = self.scoped_url(&["session", session_id, "children"])?;
        Self::send_json(self.request_url(url)).await
    }

    pub(super) async fn prompt(&self, session_id: &str, prompt: Prompt) -> Result<(), Error> {
        let url = self.scoped_url(&["session", session_id, "prompt_async"])?;
        let body = PromptBody {
            model: prompt.model,
            agent: prompt.agent,
            parts: [TextPartInput {
                kind: "text",
                text: prompt.text,
            }],
        };
        Self::send_empty(self.request_url_method(Method::POST, url).json(&body)).await
    }

    pub(super) async fn get_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, Error> {
        let mut url = self.url(&["session", session_id, "message"])?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(directory) = &self.directory {
                query.append_pair("directory", directory);
            }
            query.append_pair("limit", &limit.to_string());
        }
        Self::send_json(self.request_url(url)).await
    }

    pub(super) async fn get_sessions(&self) -> Result<Vec<Session>, Error> {
        let mut request = self.request("session")?;
        if let Some(directory) = &self.directory {
            request = request.query(&[("directory", directory)]);
        }
        Self::send_json(request).await
    }

    pub(super) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        Self::send_json(self.request(path)?).await
    }

    fn request(&self, path: &str) -> Result<reqwest::RequestBuilder, Error> {
        let url = self.base_url.join(path)?;
        Ok(self.request_url(url))
    }

    pub(super) fn request_url(&self, url: Url) -> reqwest::RequestBuilder {
        self.request_url_method(Method::GET, url)
    }

    pub(super) fn request_url_method(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let request = self.http.request(method, url);
        match (&self.username, &self.password) {
            (Some(username), Some(password)) => request.basic_auth(username, Some(password)),
            _ => request,
        }
    }

    pub(super) fn url(&self, segments: &[&str]) -> Result<Url, Error> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|()| Error::InvalidBaseUrl)?;
        path.pop_if_empty();
        path.extend(segments);
        drop(path);
        Ok(url)
    }

    fn scoped_url(&self, segments: &[&str]) -> Result<Url, Error> {
        let mut url = self.url(segments)?;
        if let Some(directory) = &self.directory {
            url.query_pairs_mut().append_pair("directory", directory);
        }
        Ok(url)
    }

    async fn send_json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, Error> {
        let response = request
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "response body unavailable".into());
            return Err(Error::Http { status, message });
        }
        Ok(response.json().await?)
    }

    async fn send_empty(request: reqwest::RequestBuilder) -> Result<(), Error> {
        let response = request
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "response body unavailable".into());
            return Err(Error::Http { status, message });
        }
        Ok(())
    }
}
