use std::time::Duration;

use ureq::Agent;

use super::assistant::Transport;

const TIMEOUT: Duration = Duration::from_mins(2);

pub struct HttpTransport {
    agent: Agent,
}

impl HttpTransport {
    pub fn new() -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for HttpTransport {
    fn post(
        &self,
        url: &str,
        headers: &[(&'static str, String)],
        body: &str,
    ) -> Result<(u16, String), String> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        let mut response = request.send(body).map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|error| error.to_string())?;
        Ok((status, text))
    }
}
