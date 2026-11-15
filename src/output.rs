//! Output: text and the JSON envelope, safe when a pipe closes (T-004; R-OUT-1, R-OUT-5, R-OUT-7).

use std::io::{self, Write};

use serde::Serialize;

use crate::error::{ErrorCode, ErrorDetails, TodoError};

/// Builds the versioned JSON schema name `todo.<command>.v1` (R-OUT-1, NFR-8).
#[must_use]
pub fn schema_name(command: &str) -> String {
    format!("todo.{command}.v1")
}

#[derive(Serialize)]
struct JsonErrorBody<'a> {
    code: ErrorCode,
    message: &'a str,
    #[serde(flatten)]
    details: &'a ErrorDetails,
}

#[derive(Serialize)]
struct JsonOkEnvelope<'a, T: Serialize> {
    schema: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<&'a str>,
    data: &'a T,
    warnings: &'a [String],
}

#[derive(Serialize)]
struct JsonErrEnvelope<'a> {
    schema: String,
    ok: bool,
    error: JsonErrorBody<'a>,
    warnings: &'a [String],
}

/// Manages standard output, standard error, warnings, and `--json` envelopes.
///
/// Never uses `println!` or `eprintln!`. If the reader of stdout closes the pipe
/// (`BrokenPipe`), `Output` marks `broken_pipe` and stops writing quietly so the
/// process can exit `0` (R-OUT-7).
pub struct Output {
    json: bool,
    warnings: Vec<String>,
    dry_run_diff: Option<String>,
    broken_pipe: bool,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

impl Output {
    /// Creates an [`Output`] connected to process `stdout` and `stderr`.
    #[must_use]
    pub fn stdio(json: bool) -> Self {
        Self {
            json,
            warnings: Vec::new(),
            dry_run_diff: None,
            broken_pipe: false,
            stdout: Box::new(io::stdout()),
            stderr: Box::new(io::stderr()),
        }
    }

    /// Creates an [`Output`] with custom writers (primarily for tests).
    #[must_use]
    pub fn with_writers(json: bool, stdout: Box<dyn Write>, stderr: Box<dyn Write>) -> Self {
        Self {
            json,
            warnings: Vec::new(),
            dry_run_diff: None,
            broken_pipe: false,
            stdout,
            stderr,
        }
    }

    /// Records a `--dry-run` unified diff to include in the `--json` envelope (`R-EDIT-10`).
    pub fn set_dry_run_diff(&mut self, diff: String) {
        self.dry_run_diff = Some(diff);
    }

    /// Returns `true` if `--json` mode is active.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        self.json
    }

    /// Returns `true` if stdout closed with `BrokenPipe` (R-OUT-7).
    #[must_use]
    pub const fn broken_pipe(&self) -> bool {
        self.broken_pipe
    }

    /// Returns the collected warnings.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Records a warning.
    ///
    /// In text mode, writes `warning: <msg>` to stderr immediately.
    /// In `--json` mode, buffers the warning to include in the JSON envelope's `warnings` array.
    pub fn warn(&mut self, message: impl Into<String>) {
        let msg = message.into();
        if self.json {
            self.warnings.push(msg);
        } else {
            self.warnings.push(msg.clone());
            self.write_stderr_line(&format!("warning: {msg}"));
        }
    }

    /// Writes plain text to stdout (no trailing newline added).
    ///
    /// Ignored in `--json` mode or if stdout already hit `BrokenPipe`.
    pub fn write_str(&mut self, text: &str) {
        if self.json || self.broken_pipe {
            return;
        }
        self.write_stdout_bytes(text.as_bytes());
    }

    /// Writes a single line to stdout, appending `\n`.
    ///
    /// Ignored in `--json` mode or if stdout already hit `BrokenPipe`.
    pub fn write_line(&mut self, line: &str) {
        if self.json || self.broken_pipe {
            return;
        }
        let mut buf = Vec::with_capacity(line.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        self.write_stdout_bytes(&buf);
    }

    /// Emits a successful `--json` envelope on stdout.
    ///
    /// Does nothing in text mode.
    pub fn emit_json_ok<T: Serialize>(&mut self, command: &str, data: &T) {
        if !self.json || self.broken_pipe {
            return;
        }
        let envelope = JsonOkEnvelope {
            schema: schema_name(command),
            ok: true,
            dry_run: self.dry_run_diff.as_ref().map(|_| true),
            diff: self.dry_run_diff.as_deref(),
            data,
            warnings: &self.warnings,
        };
        if let Ok(mut serialized) = serde_json::to_vec(&envelope) {
            serialized.push(b'\n');
            self.write_stdout_bytes(&serialized);
        }
    }

    /// Emits an error: as a JSON envelope on stdout in `--json` mode, or to stderr in text mode.
    pub fn emit_error(&mut self, command: &str, err: &TodoError) {
        if self.json {
            if self.broken_pipe {
                return;
            }
            let envelope = JsonErrEnvelope {
                schema: schema_name(command),
                ok: false,
                error: JsonErrorBody {
                    code: err.code,
                    message: &err.message,
                    details: &err.details,
                },
                warnings: &self.warnings,
            };
            if let Ok(mut serialized) = serde_json::to_vec(&envelope) {
                serialized.push(b'\n');
                self.write_stdout_bytes(&serialized);
            }
        } else {
            self.write_stderr_line(&format!("error: {err}"));
        }
    }

    /// Flushes stdout and stderr, ignoring `BrokenPipe`.
    pub fn flush(&mut self) {
        if !self.broken_pipe
            && let Err(e) = self.stdout.flush()
            && e.kind() == io::ErrorKind::BrokenPipe
        {
            self.broken_pipe = true;
        }
        let _ = self.stderr.flush();
    }

    fn write_stdout_bytes(&mut self, bytes: &[u8]) {
        if let Err(e) = self.stdout.write_all(bytes) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                self.broken_pipe = true;
            }
            return;
        }
        if let Err(e) = self.stdout.flush()
            && e.kind() == io::ErrorKind::BrokenPipe
        {
            self.broken_pipe = true;
        }
    }

    fn write_stderr_line(&mut self, line: &str) {
        let mut buf = Vec::with_capacity(line.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        let _ = self.stderr.write_all(&buf);
        let _ = self.stderr.flush();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use serde_json::json;

    use super::*;

    #[derive(Clone, Default)]
    struct SharedBuf(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct BrokenPipeWriter;

    impl Write for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn schema_name_is_versioned() {
        assert_eq!(schema_name("list"), "todo.list.v1");
        assert_eq!(schema_name("help"), "todo.help.v1");
    }

    #[test]
    fn json_ok_envelope_includes_schema_and_warnings() {
        let out_buf = SharedBuf::default();
        let err_buf = SharedBuf::default();
        let mut out =
            Output::with_writers(true, Box::new(out_buf.clone()), Box::new(err_buf.clone()));
        out.warn("stale lock removed");
        out.emit_json_ok("list", &json!({ "count": 2 }));

        let stdout_str = String::from_utf8(out_buf.0.borrow().clone()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(stdout_str.trim()).unwrap();
        assert_eq!(parsed["schema"], "todo.list.v1");
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["data"]["count"], 2);
        assert_eq!(parsed["warnings"], json!(["stale lock removed"]));
        assert!(err_buf.0.borrow().is_empty());
    }

    #[test]
    fn json_error_envelope_includes_code_message_and_details() {
        let out_buf = SharedBuf::default();
        let err_buf = SharedBuf::default();
        let mut out =
            Output::with_writers(true, Box::new(out_buf.clone()), Box::new(err_buf.clone()));
        let err = TodoError::new(ErrorCode::AlreadyClaimed, "task is claimed by alice")
            .with_task("T-012")
            .with_owner("alice");
        out.emit_error("claim", &err);

        let stdout_str = String::from_utf8(out_buf.0.borrow().clone()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(stdout_str.trim()).unwrap();
        assert_eq!(parsed["schema"], "todo.claim.v1");
        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["error"]["code"], "ALREADY_CLAIMED");
        assert_eq!(parsed["error"]["message"], "task is claimed by alice");
        assert_eq!(parsed["error"]["task"], "T-012");
        assert_eq!(parsed["error"]["owner"], "alice");
        assert_eq!(parsed["warnings"], json!([]));
        assert!(err_buf.0.borrow().is_empty());
    }

    #[test]
    fn text_mode_writes_results_to_stdout_and_errors_to_stderr() {
        let out_buf = SharedBuf::default();
        let err_buf = SharedBuf::default();
        let mut out =
            Output::with_writers(false, Box::new(out_buf.clone()), Box::new(err_buf.clone()));
        out.write_line("hello");
        out.warn("careful");
        out.emit_error("show", &TodoError::new(ErrorCode::NotFound, "no such task"));

        assert_eq!(
            String::from_utf8(out_buf.0.borrow().clone()).unwrap(),
            "hello\n"
        );
        assert_eq!(
            String::from_utf8(err_buf.0.borrow().clone()).unwrap(),
            "warning: careful\nerror: no such task\n"
        );
    }

    #[test]
    fn broken_pipe_sets_flag_without_panicking() {
        let err_buf = SharedBuf::default();
        let mut out = Output::with_writers(false, Box::new(BrokenPipeWriter), Box::new(err_buf));
        out.write_line("first line");
        assert!(out.broken_pipe());
        out.write_line("second line");
    }
}
