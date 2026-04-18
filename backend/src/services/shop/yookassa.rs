use std::collections::BTreeMap;

use crate::app::config::YooKassaConfig;
use anyhow::{Context, Result, anyhow};
use reqwest::{
    Url,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{CreatePaymentCommand, CreatePaymentResult};

pub const PROVIDER_NAME: &str = "yookassa";

#[derive(Clone)]
pub struct YooKassaClient {
    http: reqwest::Client,
    config: YooKassaConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct YooKassaPayment {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub paid: bool,
    #[serde(default)]
    pub test: bool,
    #[serde(default)]
    pub amount: Option<YooKassaAmount>,
    #[serde(default)]
    pub income_amount: Option<YooKassaAmount>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub confirmation: Option<YooKassaConfirmation>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub cancellation_details: Option<YooKassaCancellationDetails>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub captured_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct YooKassaAmount {
    pub value: String,
    pub currency: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct YooKassaConfirmation {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub confirmation_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct YooKassaCancellationDetails {
    #[serde(default)]
    pub party: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct YooKassaWebhookNotification {
    pub event: String,
    pub object: YooKassaWebhookObject,
}

#[derive(Clone, Debug, Deserialize)]
pub struct YooKassaWebhookObject {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
}

impl YooKassaClient {
    pub fn new(config: YooKassaConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub async fn create_payment(
        &self,
        command: CreatePaymentCommand,
    ) -> Result<CreatePaymentResult> {
        let return_url = build_return_url_with_order_id(&self.config.return_url, command.order_id)?;
        let request_payload = json!({
            "amount": {
                "value": format!("{}.00", command.amount_rub),
                "currency": "RUB",
            },
            "capture": true,
            "confirmation": {
                "type": "redirect",
                "return_url": return_url,
            },
            "description": command.description,
            "metadata": {
                "orderId": command.order_id.to_string(),
            }
        });

        let response = self
            .http
            .post(format!("{}/payments", self.config.api_base_url))
            .basic_auth(&self.config.shop_id, Some(&self.config.secret_key))
            .header("Idempotence-Key", command.order_id.to_string())
            .json(&request_payload)
            .send()
            .await
            .context("Failed to create YooKassa payment")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read YooKassa create payment response body")?;
        if !status.is_success() {
            return Err(anyhow!(
                "YooKassa create payment failed with status {}: {}",
                status,
                response_text
            ));
        }
        let payment: YooKassaPayment = serde_json::from_str(&response_text)
            .context("Failed to parse YooKassa create payment response")?;
        let response_payload: Value = serde_json::from_str(&response_text)
            .context("Failed to preserve YooKassa create payment payload")?;

        Ok(CreatePaymentResult {
            provider_payment_id: payment.id.clone(),
            checkout_token: None,
            checkout_url: payment
                .confirmation
                .as_ref()
                .and_then(|confirmation| confirmation.confirmation_url.clone()),
            request_payload,
            response_payload,
        })
    }

    pub async fn get_payment(&self, payment_id: &str) -> Result<YooKassaPayment> {
        let response = self
            .http
            .get(format!(
                "{}/payments/{}",
                self.config.api_base_url, payment_id
            ))
            .basic_auth(&self.config.shop_id, Some(&self.config.secret_key))
            .send()
            .await
            .context("Failed to fetch YooKassa payment state")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read YooKassa payment state body")?;
        if !status.is_success() {
            return Err(anyhow!(
                "YooKassa get payment failed with status {}: {}",
                status,
                response_text
            ));
        }

        serde_json::from_str(&response_text).context("Failed to parse YooKassa payment state")
    }
}

pub fn parse_webhook_notification(body: &str) -> Result<YooKassaWebhookNotification> {
    let payload: Value =
        serde_json::from_str(body).context("YooKassa webhook payload must be valid JSON")?;
    let kind = payload
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("YooKassa webhook payload must include string field 'type'"))?;
    if kind != "notification" {
        return Err(anyhow!(
            "YooKassa webhook payload must have type='notification'"
        ));
    }
    let event = payload
        .get("event")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("YooKassa webhook payload must include string field 'event'"))?;
    let object = payload
        .get("object")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("YooKassa webhook payload must include object field 'object'"))?;
    let payment_id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("YooKassa webhook payload must include string field 'object.id'"))?;
    if payment_id.trim().is_empty() {
        return Err(anyhow!(
            "YooKassa webhook payload field 'object.id' must not be empty"
        ));
    }
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(YooKassaWebhookNotification {
        event: event.to_string(),
        object: YooKassaWebhookObject {
            id: payment_id.to_string(),
            status,
        },
    })
}

fn build_return_url_with_order_id(base_return_url: &str, order_id: uuid::Uuid) -> Result<String> {
    let mut url =
        Url::parse(base_return_url).context("YOOKASSA_RETURN_URL must be a valid absolute URL")?;
    url.query_pairs_mut()
        .append_pair("orderId", &order_id.to_string());
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::build_return_url_with_order_id;
    use uuid::Uuid;

    #[test]
    fn appends_order_id_to_return_url() {
        let order_id = Uuid::parse_str("6d5e4f2d-b82f-4d57-a69c-7ca6a234f3bd").expect("uuid");
        let url =
            build_return_url_with_order_id("http://localhost:5173/shop/checkout/return", order_id)
                .expect("return url");
        assert_eq!(
            url,
            "http://localhost:5173/shop/checkout/return?orderId=6d5e4f2d-b82f-4d57-a69c-7ca6a234f3bd"
        );
    }

    #[test]
    fn preserves_existing_query_params_when_appending_order_id() {
        let order_id = Uuid::parse_str("6d5e4f2d-b82f-4d57-a69c-7ca6a234f3bd").expect("uuid");
        let url = build_return_url_with_order_id(
            "http://localhost:5173/shop/checkout/return?source=yookassa",
            order_id,
        )
        .expect("return url");
        assert_eq!(
            url,
            "http://localhost:5173/shop/checkout/return?source=yookassa&orderId=6d5e4f2d-b82f-4d57-a69c-7ca6a234f3bd"
        );
    }
}

pub fn payment_to_json(payment: &YooKassaPayment) -> Result<Value> {
    serde_json::to_value(payment).context("Failed to serialize YooKassa payment payload")
}

pub fn cancellation_problem(payment: &YooKassaPayment) -> String {
    let mut parts = vec!["YooKassa payment was canceled".to_string()];
    if let Some(details) = &payment.cancellation_details {
        if let Some(reason) = details.reason.as_deref() {
            parts.push(format!("reason: {reason}"));
        }
        if let Some(party) = details.party.as_deref() {
            parts.push(format!("party: {party}"));
        }
    }
    parts.join("; ")
}

pub fn extract_request_headers(headers: &HeaderMap) -> Value {
    let mut result = serde_json::Map::new();
    for (name, value) in headers {
        result.insert(
            name.to_string(),
            Value::String(header_value_to_string(value)),
        );
    }
    Value::Object(result)
}

fn header_value_to_string(value: &HeaderValue) -> String {
    value.to_str().unwrap_or("<non-utf8>").to_string()
}
