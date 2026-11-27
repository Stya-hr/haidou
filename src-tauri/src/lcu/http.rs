use super::auth::Credentials;
use anyhow::{bail, Result};
use base64::Engine;
use reqwest::Method;
use serde_json::Value;
use std::time::Duration;

/// LCU REST 客户端，只连 127.0.0.1。
#[derive(Clone)]
pub struct LcuClient {
    base: String,
    auth: String,
    http: reqwest::Client,
}

impl LcuClient {
    pub fn new(c: &Credentials) -> Result<Self> {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", c.password));
        Ok(Self {
            base: format!("https://127.0.0.1:{}", c.port),
            auth: format!("Basic {token}"),
            http: local_https_client(Duration::from_secs(3))?,
        })
    }

    pub fn auth_header(&self) -> &str {
        &self.auth
    }

    pub async fn request(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let mut req = self
            .http
            .request(method.clone(), format!("{}{}", self.base, path))
            .header("Authorization", &self.auth);
        if let Some(b) = body {
            req = req.json(b);
        }
        let r = req.send().await?;
        let status = r.status();
        let text = r.text().await.unwrap_or_default();
        if !status.is_success() {
            // LCU 的错误体里一般带 message 字段，透出给用户看
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["message"].as_str().map(String::from))
                .unwrap_or(text);
            bail!("{method} {path} -> {status} {msg}");
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        self.request(Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str) -> Result<()> {
        self.request(Method::POST, path, None).await.map(|_| ())
    }
}

/// 客户端和游戏的本地接口用的是 Riot 自签证书，且只监听 127.0.0.1，
/// 因此对本地连接放宽证书校验；同时禁用系统代理，防止本地请求被代理转发。
pub fn local_https_client(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .timeout(timeout)
        .build()?)
}
