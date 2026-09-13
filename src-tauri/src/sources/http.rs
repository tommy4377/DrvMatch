use std::{io::Read, thread, time::Duration};

use reqwest::{
    StatusCode,
    blocking::{RequestBuilder, Response},
};

const RETRY_DELAY: Duration = Duration::from_millis(250);

pub fn get_with_retry(
    mut request: impl FnMut() -> RequestBuilder,
    label: &str,
) -> Result<Response, String> {
    for attempt in 0..2 {
        match request().send() {
            Ok(response) if response.status().is_success() => return Ok(response),
            Ok(response) => {
                let status = response.status();
                if attempt == 0 && retryable_status(status) {
                    thread::sleep(RETRY_DELAY);
                    continue;
                }
                return Err(format!("{label} returned HTTP {status}."));
            }
            Err(error) => {
                if attempt == 0 && (error.is_connect() || error.is_timeout() || error.is_request())
                {
                    thread::sleep(RETRY_DELAY);
                    continue;
                }
                return Err(format!("{label} failed: {error}"));
            }
        }
    }
    unreachable!("the bounded request loop always returns")
}

fn retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

pub fn read_limited(mut response: Response, limit: u64, label: &str) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(format!("{label} exceeds the {limit}-byte metadata limit."));
    }
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read {label}: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("{label} exceeds the {limit}-byte metadata limit."));
    }
    Ok(bytes)
}

pub fn read_text_limited(response: Response, limit: u64, label: &str) -> Result<String, String> {
    Ok(String::from_utf8_lossy(&read_limited(response, limit, label)?).into_owned())
}

#[cfg(test)]
mod tests {
    use super::retryable_status;
    use reqwest::StatusCode;

    #[test]
    fn retries_only_transient_http_failures() {
        assert!(retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!retryable_status(StatusCode::BAD_REQUEST));
        assert!(!retryable_status(StatusCode::NOT_FOUND));
    }
}
