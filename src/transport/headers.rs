/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 8/9/24
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use tracing::{debug, warn};
use reqwest::header::HeaderMap;

pub (crate) enum Version {
    V1,
    V2,
    V3,
}

#[derive(Debug)]
pub(crate) struct SecurityHeaders {
    pub(crate) cst: Option<String>,
    pub(crate) x_security_token: Option<String>,
    pub(crate) ig_account_id: Option<String>,
    pub(crate) authorization: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) x_ig_api_key: Option<String>,
}

impl SecurityHeaders {
    pub(crate) fn new(cst: Option<String>,
                      x_security_token: Option<String>,
                      ig_account_id: Option<String>,
                      authorization: Option<String>,
                      version: Option<String>,
                      x_ig_api_key: Option<String>) -> Self {
        Self {
            cst,
            x_security_token,
            ig_account_id,
            authorization,
            version,
            x_ig_api_key,
        }
    }

    /// Retrieves the V1 version of headers needed for API requests.
    ///
    /// This function constructs and returns a `HashMap` containing several headers relevant
    /// to the V1 version of the API.
    ///
    /// # Returns
    ///
    /// A `HashMap<String, String>` where:
    /// - The key `"Version"` maps to the value `"1"`.
    /// - The key `"X-IG-API-KEY"` maps to the value of the field `x_ig_api_key`.
    /// - The key `"CST"` maps to the value of the field `cst`.
    /// - The key `"X-SECURITY-TOKEN"` maps to the value of the field `x_security_token`.
    ///
    /// # Debugging
    ///
    /// Debug statements log the headers for V1.
    pub(crate) fn get_v1(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "1".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("CST".to_string(), self.cst.to_string());
        headers.insert("X-SECURITY-TOKEN".to_string(), self.x_security_token.to_string());
        debug!("Headers V1: {:?}", headers);
        headers
    }

    /// Generates a HashMap of headers for version 2 of the API.
    ///
    /// This function collects necessary headers needed for the API version 2 request
    /// and returns them in a HashMap. The headers include:
    ///
    /// - "Version": Fixed to "2".
    /// - "X-IG-API-KEY": API key for authentication.
    /// - "IG-ACCOUNT-ID": The IG account ID.
    /// - "Authorization": The authorization token.
    ///
    /// Also, logs the headers for debugging purposes.
    ///
    /// # Returns
    ///
    /// A `HashMap` where each key is a `String` representing the header name,
    /// and each value is a `String` representing the header value.
    ///
    pub(crate) fn get_v2(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "2".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("IG-ACCOUNT-ID".to_string(), self.ig_account_id.to_string());
        headers.insert("Authorization".to_string(), self.authorization.to_string());
        debug!("Headers V2: {:?}", headers);
        headers
    }

    /// Generates a set of headers needed for Version 3 API requests.
    ///
    /// This function collects various necessary headers required
    /// to make API requests compliant with Version 3.
    /// The headers include the API version, API key, account ID,
    /// and authorization token.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the keys and values are both `String` types
    /// representing the header field names and their associated values.
    ///
    /// This will return a `HashMap` containing the following headers:
    /// - "Version": "3"
    /// - "X-IG-API-KEY": API key from the struct instance (`self`)
    /// - "IG-ACCOUNT-ID": Account ID from the struct instance (`self`)
    /// - "Authorization": Authorization token from the struct instance (`self`)
    ///
    /// A debug log entry is created displaying the headers.
    pub(crate) fn get_v3(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "3".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("IG-ACCOUNT-ID".to_string(), self.ig_account_id.to_string());
        headers.insert("Authorization".to_string(), self.authorization.to_string());
        debug!("Headers V3: {:?}", headers);
        headers
    }

    /// Updates the internal headers of the struct based on the provided `HeaderMap`.
    ///
    /// This function iterates over the key-value pairs in the `HeaderMap` and updates
    /// the corresponding internal fields of the struct based on the header names. The
    /// recognized headers and their respective fields are as follows:
    ///
    /// - "CST" -> `self.cst`
    /// - "X-SECURITY-TOKEN" -> `self.x_security_token`
    /// - "IG-ACCOUNT-ID" -> `self.ig_account_id`
    /// - "AUTHORIZATION" -> `self.authorization`
    /// - "VERSION" -> `self.version`
    /// - "X-IG-API-KEY" -> `self.x_ig_api_key`
    ///
    /// If an unknown header is encountered, a warning is logged.
    ///
    /// # Arguments
    ///
    /// * `headers` - A `HeaderMap` containing the headers to be updated.
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Returns `Ok(())` if the headers are updated successfully,
    /// or an error if there is an issue converting the header value to a string.
    ///
    /// # Errors
    ///
    /// * `anyhow::Error` - An error is returned if the value of a recognized header cannot be
    /// converted to a string.
    pub(crate) fn update_headers(&mut self, headers: HeaderMap) -> anyhow::Result<()> {
        for (name, value) in headers.iter() {
            match name.as_str().to_uppercase().as_str() {
                "CST" => self.cst = Some(value.to_str()?.to_string()),
                "X-SECURITY-TOKEN" => self.x_security_token = Some(value.to_str()?.to_string()),
                "IG-ACCOUNT-ID" => self.ig_account_id = Some(value.to_str()?.to_string()),
                "AUTHORIZATION" => self.authorization = Some(value.to_str()?.to_string()),
                "VERSION" => self.version = Some(value.to_str()?.to_string()),
                "X-IG-API-KEY" => self.x_ig_api_key = Some(value.to_str()?.to_string()),
                _ => {
                    warn!("Unknown header: {} with value: {}", name, value.to_str()?);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn get_headers_from_version(&self, version: Version) -> HashMap<String, String> {
        match version {
            Version::V1 => self.get_v1(),
            Version::V2 => self.get_v2(),
            Version::V3 => self.get_v3(),
            None => {
                warn!("Unknown version: {:?}", version);
                HashMap::new()
            }
        }
    }
}

impl Display for SecurityHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"cst\":\"{}\",\"x_security_token\":\"{}\",\"ig_account_id\":\"{}\",\"authorization\":\"{}\",\"version\":\"{}\",\"x_ig_api_key\":\"{}\"}}",
            self.cst.as_deref().unwrap_or(""),
            self.x_security_token.as_deref().unwrap_or(""),
            self.ig_account_id.as_deref().unwrap_or(""),
            self.authorization.as_deref().unwrap_or(""),
            self.version.as_deref().unwrap_or(""),
            self.x_ig_api_key.as_deref().unwrap_or("")
        )
    }
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            cst: None,
            x_security_token: None,
            ig_account_id: None,
            authorization: None,
            version: None,
            x_ig_api_key: None,
        }
    }
}

#[cfg(test)]
mod tests_security_headers {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_new_security_headers() {
        let headers = SecurityHeaders::new(
            Some("cst".to_string()),
            Some("x_security_token".to_string()),
            Some("ig_account_id".to_string()),
            Some("authorization".to_string()),
            Some("version".to_string()),
            Some("x_ig_api_key".to_string()),
        );

        assert_eq!(headers.cst, Some("cst".to_string()));
        assert_eq!(headers.x_security_token, Some("x_security_token".to_string()));
        assert_eq!(headers.ig_account_id, Some("ig_account_id".to_string()));
        assert_eq!(headers.authorization, Some("authorization".to_string()));
        assert_eq!(headers.version, Some("version".to_string()));
        assert_eq!(headers.x_ig_api_key, Some("x_ig_api_key".to_string()));
    }

    #[test]
    fn test_get_v1_headers() {
        let headers = SecurityHeaders::new(
            Some("cst".to_string()),
            Some("x_security_token".to_string()),
            None,
            None,
            None,
            Some("x_ig_api_key".to_string()),
        );

        let v1_headers = headers.get_v1();

        assert_eq!(v1_headers.get("Version"), Some(&"1".to_string()));
        assert_eq!(v1_headers.get("X-IG-API-KEY"), Some(&"x_ig_api_key".to_string()));
        assert_eq!(v1_headers.get("CST"), Some(&"cst".to_string()));
        assert_eq!(v1_headers.get("X-SECURITY-TOKEN"), Some(&"x_security_token".to_string()));
    }

    #[test]
    fn test_get_v2_headers() {
        let headers = SecurityHeaders::new(
            None,
            None,
            Some("ig_account_id".to_string()),
            Some("authorization".to_string()),
            None,
            Some("x_ig_api_key".to_string()),
        );

        let v2_headers = headers.get_v2();

        assert_eq!(v2_headers.get("Version"), Some(&"2".to_string()));
        assert_eq!(v2_headers.get("X-IG-API-KEY"), Some(&"x_ig_api_key".to_string()));
        assert_eq!(v2_headers.get("IG-ACCOUNT-ID"), Some(&"ig_account_id".to_string()));
        assert_eq!(v2_headers.get("Authorization"), Some(&"authorization".to_string()));
    }

    #[test]
    fn test_get_v3_headers() {
        let headers = SecurityHeaders::new(
            None,
            None,
            Some("ig_account_id".to_string()),
            Some("authorization".to_string()),
            None,
            Some("x_ig_api_key".to_string()),
        );

        let v3_headers = headers.get_v3();

        assert_eq!(v3_headers.get("Version"), Some(&"3".to_string()));
        assert_eq!(v3_headers.get("X-IG-API-KEY"), Some(&"x_ig_api_key".to_string()));
        assert_eq!(v3_headers.get("IG-ACCOUNT-ID"), Some(&"ig_account_id".to_string()));
        assert_eq!(v3_headers.get("Authorization"), Some(&"authorization".to_string()));
    }

    #[test]
    fn test_update_headers() {
        let mut headers = SecurityHeaders::default();
        let mut header_map = HeaderMap::new();
        header_map.insert("CST", HeaderValue::from_static("new_cst"));
        header_map.insert("x-security-token", HeaderValue::from_static("new_token"));
        header_map.insert("IG-ACCOUNT-ID", HeaderValue::from_static("new_account"));
        header_map.insert("Authorization", HeaderValue::from_static("new_auth"));
        header_map.insert("Version", HeaderValue::from_static("new_version"));
        header_map.insert("X-IG-API-KEY", HeaderValue::from_static("new_api_key"));

        headers.update_headers(header_map).unwrap();

        assert_eq!(headers.cst, Some("new_cst".to_string()));
        assert_eq!(headers.x_security_token, Some("new_token".to_string()));
        assert_eq!(headers.ig_account_id, Some("new_account".to_string()));
        assert_eq!(headers.authorization, Some("new_auth".to_string()));
        assert_eq!(headers.version, Some("new_version".to_string()));
        assert_eq!(headers.x_ig_api_key, Some("new_api_key".to_string()));
    }

    #[test]
    fn test_display_implementation() {
        let headers = SecurityHeaders::new(
            Some("cst".to_string()),
            Some("token".to_string()),
            Some("account".to_string()),
            Some("auth".to_string()),
            Some("1".to_string()),
            Some("api_key".to_string()),
        );

        let display_string = format!("{}", headers);
        assert_eq!(display_string, "{\"cst\":\"cst\",\"x_security_token\":\"token\",\"ig_account_id\":\"account\",\"authorization\":\"auth\",\"version\":\"1\",\"x_ig_api_key\":\"api_key\"}");
    }

    #[test]
    fn test_default_implementation() {
        let headers = SecurityHeaders::default();

        assert_eq!(headers.cst, None);
        assert_eq!(headers.x_security_token, None);
        assert_eq!(headers.ig_account_id, None);
        assert_eq!(headers.authorization, None);
        assert_eq!(headers.version, None);
        assert_eq!(headers.x_ig_api_key, None);
    }
}