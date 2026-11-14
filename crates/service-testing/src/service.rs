// SPDX-License-Identifier: MIT

//! The service binary under test.

use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep};
use std::time::{Duration, Instant};

use crate::free_port;

/// A service binary under test, started with exactly the environment it is
/// given (plus `PATH` and cargo-llvm-cov's `LLVM_PROFILE_FILE`); stopped with
/// SIGTERM on drop.
pub struct Service {
    /// The running process.
    pub child: Child,
    /// Its HTTP port (the `port_var` it was given).
    pub port: u16,
    stdout: Output,
    stderr: Output,
}

/// One of the child's output streams, drained as it writes so it never
/// blocks on a full pipe.
struct Output {
    text: Arc<Mutex<String>>,
    reader: Option<JoinHandle<()>>,
}

impl Output {
    fn drain(mut stream: impl Read + Send + 'static) -> Self {
        let text = Arc::new(Mutex::new(String::new()));
        let sink = text.clone();
        let reader = std::thread::spawn(move || {
            let mut chunk = [0_u8; 8192];
            while let Ok(read) = stream.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                sink.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&chunk[..read]));
            }
        });
        Self {
            text,
            reader: Some(reader),
        }
    }

    /// What was written so far; everything, once `finished` (the child exited).
    fn text(&mut self, finished: bool) -> String {
        if finished && let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
        self.text.lock().unwrap().clone()
    }
}

impl Service {
    /// Starts `binary` with exactly `env` plus `port_var` set to a free port;
    /// does not wait.
    #[must_use]
    pub fn spawn(binary: &str, port_var: &str, env: &[(&str, String)]) -> Self {
        let port = free_port();
        let mut vars: HashMap<&str, String> = HashMap::from([(port_var, port.to_string())]);
        vars.extend(env.iter().map(|(key, value)| (*key, value.clone())));
        let mut child = Command::new(binary)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .envs(std::env::vars().filter(|(key, _)| key == "LLVM_PROFILE_FILE"))
            .envs(&vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = Output::drain(child.stdout.take().unwrap());
        let stderr = Output::drain(child.stderr.take().unwrap());
        Self {
            child,
            port,
            stdout,
            stderr,
        }
    }

    /// [`Service::spawn`], then waits until `/health` answers.
    #[must_use]
    pub fn start(binary: &str, port_var: &str, env: &[(&str, String)]) -> Self {
        let mut service = Self::spawn(binary, port_var, env);
        let deadline = Instant::now() + Duration::from_secs(30);
        while service.get("/health").is_none() {
            assert!(
                service.child.try_wait().unwrap().is_none(),
                "exited before serving"
            );
            assert!(Instant::now() < deadline, "never served /health");
            sleep(Duration::from_millis(50));
        }
        service
    }

    /// `GET path`: status, Content-Type and body, or None if nothing answered.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<(u16, Option<String>, String)> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        let mut response = agent
            .get(format!("http://127.0.0.1:{}{path}", self.port))
            .call()
            .ok()?;
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned());
        let body = response.body_mut().read_to_string().unwrap();
        Some((response.status().as_u16(), content_type, body))
    }

    /// `POST path` with `body` and an optional `Content-Type`: status, Content-Type, body.
    #[must_use]
    pub fn post(
        &self,
        path: &str,
        body: &[u8],
        content_type: Option<&str>,
    ) -> (u16, Option<String>, String) {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        let mut request = agent.post(format!("http://127.0.0.1:{}{path}", self.port));
        if let Some(content_type) = content_type {
            request = request.header("Content-Type", content_type);
        }
        let mut response = request.send(body).unwrap();
        let received = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned());
        let text = response.body_mut().read_to_string().unwrap();
        (response.status().as_u16(), received, text)
    }

    /// `POST path` with a JSON body.
    #[must_use]
    pub fn post_json(&self, path: &str, json: &str) -> (u16, Option<String>, String) {
        self.post(path, json.as_bytes(), Some("application/json"))
    }

    /// Any request without a body: status, all headers (lower-case names), body.
    #[must_use]
    pub fn request(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
    ) -> (u16, Vec<(String, String)>, String) {
        self.request_with_body(method, path, headers, "")
    }

    /// Any request: method, path, headers, body → status, all headers (lower-case names), body.
    /// Redirects are returned, not followed.
    #[must_use]
    pub fn request_with_body(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, Vec<(String, String)>, String) {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .allow_non_standard_methods(true)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        let mut builder = ureq::http::Request::builder()
            .method(method)
            .uri(format!("http://127.0.0.1:{}{path}", self.port));
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let mut response = agent.run(builder.body(body.to_owned()).unwrap()).unwrap();
        let received = response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.to_str().unwrap().to_owned()))
            .collect();
        let text = if method == "HEAD" {
            String::new()
        } else {
            response.body_mut().read_to_string().unwrap()
        };
        (response.status().as_u16(), received, text)
    }

    /// Waits for the process to exit by itself.
    pub fn wait_exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "did not exit");
            sleep(Duration::from_millis(20));
        }
    }

    /// Sends SIGTERM.
    pub fn terminate(&self) {
        let status = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    /// What it has written to stderr so far (all of it once it has exited).
    pub fn stderr(&mut self) -> String {
        let exited = self.child.try_wait().unwrap().is_some();
        self.stderr.text(exited)
    }

    /// What it has written to stdout so far (all of it once it has exited).
    pub fn stdout(&mut self) -> String {
        let exited = self.child.try_wait().unwrap().is_some();
        self.stdout.text(exited)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.terminate();
            let _ = self.wait_exit();
        }
    }
}
