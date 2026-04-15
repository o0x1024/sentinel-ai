use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerificationParameterMutation {
    #[serde(default)]
    pub parameter: String,
    #[serde(default = "default_mutation_kind")]
    pub mutation_kind: String,
}

fn default_mutation_kind() -> String {
    "set_negative_one".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedVerificationMutation {
    pub location: String,
    pub selector: String,
    pub mutation_kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationMutationSelectors {
    pub query: Vec<String>,
    pub json_body: Vec<String>,
    pub form_body: Vec<String>,
    pub legacy: Vec<String>,
}

pub fn mutate_business_parameter_request(
    url: &str,
    body: Option<&str>,
    mutations: &[VerificationParameterMutation],
    selectors: &VerificationMutationSelectors,
) -> Result<(String, Option<String>, Vec<AppliedVerificationMutation>)> {
    if mutations.is_empty() {
        return Err(anyhow!(
            "Business-parameter mutation strategy requires at least one parameter mutation"
        ));
    }

    let mut current_url = url.to_string();
    let mut current_body = body.map(str::to_string);
    let mut applied_mutations = Vec::with_capacity(mutations.len());
    for mutation in mutations {
        let (next_url, next_body, applied_mutation) = apply_single_business_parameter_mutation(
            &current_url,
            current_body.as_deref(),
            mutation,
            selectors,
        )?;
        current_url = next_url;
        current_body = next_body;
        applied_mutations.push(applied_mutation);
    }

    Ok((current_url, current_body, applied_mutations))
}

fn apply_single_business_parameter_mutation(
    url: &str,
    body: Option<&str>,
    mutation: &VerificationParameterMutation,
    selectors: &VerificationMutationSelectors,
) -> Result<(String, Option<String>, AppliedVerificationMutation)> {
    let query_keys = prioritized_parameter_keys(mutation, &selectors.query, &selectors.legacy);
    if let Some((mutated_url, selector)) =
        mutate_url_query(url, &query_keys, &mutation.mutation_kind)?
    {
        return Ok((
            mutated_url,
            body.map(str::to_string),
            AppliedVerificationMutation {
                location: "query".to_string(),
                selector,
                mutation_kind: mutation.mutation_kind.clone(),
            },
        ));
    }

    let json_body_keys =
        prioritized_parameter_keys(mutation, &selectors.json_body, &selectors.legacy);
    if let Some((mutated_body, selector)) =
        mutate_json_body(body, &json_body_keys, &mutation.mutation_kind)?
    {
        return Ok((
            url.to_string(),
            Some(mutated_body),
            AppliedVerificationMutation {
                location: "jsonBody".to_string(),
                selector,
                mutation_kind: mutation.mutation_kind.clone(),
            },
        ));
    }

    let form_body_keys =
        prioritized_parameter_keys(mutation, &selectors.form_body, &selectors.legacy);
    if let Some((mutated_body, selector)) =
        mutate_form_body(body, &form_body_keys, &mutation.mutation_kind)?
    {
        return Ok((
            url.to_string(),
            Some(mutated_body),
            AppliedVerificationMutation {
                location: "formBody".to_string(),
                selector,
                mutation_kind: mutation.mutation_kind.clone(),
            },
        ));
    }

    Err(anyhow!(
        "Verification plan requested business-parameter mutation but no target parameter could be mutated"
    ))
}

fn prioritized_parameter_keys(
    mutation: &VerificationParameterMutation,
    explicit_keys: &[String],
    legacy_keys: &[String],
) -> Vec<String> {
    let mut keys = Vec::new();
    if !mutation.parameter.trim().is_empty() {
        keys.push(mutation.parameter.trim().to_string());
    }
    for candidate in explicit_keys.iter().chain(legacy_keys.iter()) {
        let trimmed = candidate.trim();
        if trimmed.is_empty() || keys.iter().any(|item| item == trimmed) {
            continue;
        }
        keys.push(trimmed.to_string());
    }
    keys
}

fn mutate_url_query(
    url: &str,
    parameter_keys: &[String],
    mutation_kind: &str,
) -> Result<Option<(String, String)>> {
    let mut parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let query_items = parsed.query_pairs().collect::<Vec<_>>();
    if query_items.is_empty() {
        return Ok(None);
    }

    let mut updated = Vec::with_capacity(query_items.len());
    let mut mutated = false;
    let mut mutated_selector = None;
    for (key, value) in query_items {
        let key_text = key.to_string();
        if !mutated && parameter_matches(&key_text, parameter_keys) {
            mutated_selector = Some(key_text.clone());
            if mutation_kind == "remove_parameter" {
                mutated = true;
                continue;
            }
            updated.push((key_text, apply_mutation_to_scalar(&value, mutation_kind)?));
            mutated = true;
        } else {
            updated.push((key_text, value.to_string()));
        }
    }

    if !mutated {
        return Ok(None);
    }

    parsed.query_pairs_mut().clear().extend_pairs(updated);
    Ok(Some((
        parsed.to_string(),
        mutated_selector.unwrap_or_default(),
    )))
}

fn mutate_json_body(
    body: Option<&str>,
    parameter_keys: &[String],
    mutation_kind: &str,
) -> Result<Option<(String, String)>> {
    let Some(body) = body else {
        return Ok(None);
    };
    let mut value: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(map) = value.as_object_mut() else {
        return Ok(None);
    };

    for key in parameter_keys {
        if mutation_kind == "remove_parameter" {
            if map.remove(key).is_some() {
                return Ok(Some((serde_json::to_string(&value)?, key.clone())));
            }
            continue;
        }
        let Some(field) = map.get_mut(key) else {
            continue;
        };
        *field = apply_mutation_to_value(field, mutation_kind)?;
        return Ok(Some((serde_json::to_string(&value)?, key.clone())));
    }

    Ok(None)
}

fn mutate_form_body(
    body: Option<&str>,
    parameter_keys: &[String],
    mutation_kind: &str,
) -> Result<Option<(String, String)>> {
    let Some(raw) = body else {
        return Ok(None);
    };
    let mut pairs = url::form_urlencoded::parse(raw.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return Ok(None);
    }

    let mut mutated = false;
    let mut mutated_selector = None;
    let mut mutation_error = None;
    pairs.retain_mut(|(key, value)| {
        if !mutated && parameter_matches(key, parameter_keys) {
            mutated_selector = Some(key.clone());
            if mutation_kind == "remove_parameter" {
                mutated = true;
                return false;
            }
            *value = match apply_mutation_to_scalar(value, mutation_kind) {
                Ok(next) => next,
                Err(error) => {
                    mutation_error = Some(error);
                    return true;
                }
            };
            mutated = true;
            return true;
        }
        true
    });

    if let Some(error) = mutation_error {
        return Err(error);
    }

    if !mutated {
        return Ok(None);
    }

    Ok(Some((
        pairs
            .into_iter()
            .fold(
                url::form_urlencoded::Serializer::new(String::new()),
                |mut serializer, (key, value)| {
                    serializer.append_pair(&key, &value);
                    serializer
                },
            )
            .finish(),
        mutated_selector.unwrap_or_default(),
    )))
}

fn parameter_matches(key: &str, parameter_keys: &[String]) -> bool {
    parameter_keys
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(key))
}

fn apply_mutation_to_value(value: &Value, mutation_kind: &str) -> Result<Value> {
    match value {
        Value::Number(number) => apply_mutation_to_json_number(number, mutation_kind),
        Value::String(text) => Ok(Value::String(apply_mutation_to_scalar(
            text,
            mutation_kind,
        )?)),
        _ => Err(anyhow!(
            "Unsupported business-parameter mutation target type: {:?}",
            value
        )),
    }
}

fn apply_mutation_to_json_number(
    number: &serde_json::Number,
    mutation_kind: &str,
) -> Result<Value> {
    let current = number
        .as_i64()
        .ok_or_else(|| anyhow!("Unsupported numeric mutation target: {}", number))?;
    let next = match mutation_kind {
        "set_negative_one" => return Ok(Value::Number((-1).into())),
        "set_zero" => return Ok(Value::Number(0.into())),
        "set_one" => return Ok(Value::Number(1.into())),
        "increment_one" => current + 1,
        "set_empty_string" => return Ok(Value::String(String::new())),
        "remove_parameter" => {
            return Err(anyhow!(
                "remove_parameter should be handled before value mutation"
            ))
        }
        other => {
            return Err(anyhow!(
                "Unsupported business-parameter mutation kind: {}",
                other
            ))
        }
    };
    Ok(Value::Number(next.into()))
}

fn apply_mutation_to_scalar(value: &str, mutation_kind: &str) -> Result<String> {
    match mutation_kind {
        "set_negative_one" => Ok("-1".to_string()),
        "set_zero" => Ok("0".to_string()),
        "set_one" => Ok("1".to_string()),
        "increment_one" => increment_scalar(value),
        "set_empty_string" => Ok(String::new()),
        "remove_parameter" => Err(anyhow!(
            "remove_parameter should be handled before scalar mutation"
        )),
        other => Err(anyhow!(
            "Unsupported business-parameter mutation kind: {}",
            other
        )),
    }
}

fn increment_scalar(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let parsed = trimmed.parse::<i64>().map_err(|_| {
        anyhow!(
            "increment_one requires an integer-like scalar target: {}",
            value
        )
    })?;
    Ok((parsed + 1).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutates_quantity_in_json_body() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some(r#"{"quantity":1,"productId":"sku-1"}"#),
            &[VerificationParameterMutation {
                parameter: "quantity".to_string(),
                mutation_kind: "set_negative_one".to_string(),
            }],
            &VerificationMutationSelectors {
                json_body: vec!["quantity".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation to succeed");

        assert_eq!(applied_mutations.len(), 1);
        assert_eq!(url, "https://shop.test/api/checkout");
        let body = body.expect("mutated body");
        let parsed: Value = serde_json::from_str(&body).expect("valid json");
        assert_eq!(
            parsed,
            serde_json::json!({
                "quantity": -1,
                "productId": "sku-1"
            })
        );
    }

    #[test]
    fn removes_parameter_from_json_body() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some(r#"{"coupon":"SAVE10","productId":"sku-1"}"#),
            &[VerificationParameterMutation {
                parameter: "coupon".to_string(),
                mutation_kind: "remove_parameter".to_string(),
            }],
            &VerificationMutationSelectors {
                json_body: vec!["coupon".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation to succeed");

        assert_eq!(applied_mutations.len(), 1);
        assert_eq!(url, "https://shop.test/api/checkout");
        let body = body.expect("mutated body");
        let parsed: Value = serde_json::from_str(&body).expect("valid json");
        assert_eq!(
            parsed,
            serde_json::json!({
                "productId": "sku-1"
            })
        );
    }

    #[test]
    fn increments_query_parameter() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout?quantity=1",
            None,
            &[VerificationParameterMutation {
                parameter: "quantity".to_string(),
                mutation_kind: "increment_one".to_string(),
            }],
            &VerificationMutationSelectors {
                query: vec!["quantity".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation to succeed");

        assert_eq!(applied_mutations.len(), 1);
        assert_eq!(url, "https://shop.test/api/checkout?quantity=2");
        assert!(body.is_none());
    }

    #[test]
    fn sets_form_parameter_to_zero() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some("quantity=3&productId=sku-1"),
            &[VerificationParameterMutation {
                parameter: "quantity".to_string(),
                mutation_kind: "set_zero".to_string(),
            }],
            &VerificationMutationSelectors {
                form_body: vec!["quantity".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation to succeed");

        assert_eq!(applied_mutations.len(), 1);
        assert_eq!(url, "https://shop.test/api/checkout");
        assert_eq!(body.as_deref(), Some("quantity=0&productId=sku-1"));
    }

    #[test]
    fn sets_form_parameter_to_one() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some("price=133700&quantity=1"),
            &[VerificationParameterMutation {
                parameter: "price".to_string(),
                mutation_kind: "set_one".to_string(),
            }],
            &VerificationMutationSelectors {
                form_body: vec!["price".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation to succeed");

        assert_eq!(applied_mutations.len(), 1);
        assert_eq!(url, "https://shop.test/api/checkout");
        assert_eq!(body.as_deref(), Some("price=1&quantity=1"));
    }

    #[test]
    fn applies_multiple_json_mutations_in_order() {
        let (url, body, applied_mutations) = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some(r#"{"quantity":2,"coupon":"SAVE10","productId":"sku-1"}"#),
            &[
                VerificationParameterMutation {
                    parameter: "coupon".to_string(),
                    mutation_kind: "remove_parameter".to_string(),
                },
                VerificationParameterMutation {
                    parameter: "quantity".to_string(),
                    mutation_kind: "set_zero".to_string(),
                },
            ],
            &VerificationMutationSelectors {
                json_body: vec!["coupon".to_string(), "quantity".to_string()],
                ..VerificationMutationSelectors::default()
            },
        )
        .expect("mutation sequence to succeed");

        assert_eq!(applied_mutations.len(), 2);
        assert_eq!(applied_mutations[0].selector, "coupon");
        assert_eq!(applied_mutations[1].selector, "quantity");
        assert_eq!(url, "https://shop.test/api/checkout");
        let body = body.expect("mutated body");
        let parsed: Value = serde_json::from_str(&body).expect("valid json");
        assert_eq!(
            parsed,
            serde_json::json!({
                "quantity": 0,
                "productId": "sku-1"
            })
        );
    }

    #[test]
    fn does_not_guess_fallback_business_parameter_names() {
        let result = mutate_business_parameter_request(
            "https://shop.test/api/checkout",
            Some(r#"{"quantity":1,"productId":"sku-1"}"#),
            &[VerificationParameterMutation {
                parameter: String::new(),
                mutation_kind: "set_negative_one".to_string(),
            }],
            &VerificationMutationSelectors::default(),
        );

        assert!(result.is_err());
    }
}
