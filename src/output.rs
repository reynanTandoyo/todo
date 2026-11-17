//! Output: text and the JSON envelope, safe when a pipe closes (T-004, T-068; R-OUT-1, R-OUT-5, R-OUT-7, R-OUT-9).

use std::io::{self, IsTerminal, Write};

use serde::Serialize;

use crate::error::{ErrorCode, ErrorDetails, ExitCode, TodoError};

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

/// Returns `true` when ANSI colour should be enabled: only on a terminal and never with `NO_COLOR` (`R-OUT-9`).
#[must_use]
pub const fn should_use_color(is_terminal: bool, no_color: bool) -> bool {
    is_terminal && !no_color
}

fn no_color_env() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

/// Manages standard output, standard error, warnings, and `--json` envelopes.
///
/// Never uses `println!` or `eprintln!`. If the reader of stdout closes the pipe
/// (`BrokenPipe`), `Output` marks `broken_pipe` and stops writing quietly so the
/// process can exit `0` (R-OUT-7).
pub struct Output {
    json: bool,
    stdout_color: bool,
    stderr_color: bool,
    warnings: Vec<String>,
    dry_run_diff: Option<String>,
    broken_pipe: bool,
    exit_override: Option<ExitCode>,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

impl Output {
    /// Creates an [`Output`] connected to process `stdout` and `stderr`.
    #[must_use]
    pub fn stdio(json: bool) -> Self {
        let no_color = no_color_env();
        Self {
            json,
            stdout_color: !json && should_use_color(io::stdout().is_terminal(), no_color),
            stderr_color: should_use_color(io::stderr().is_terminal(), no_color),
            warnings: Vec::new(),
            dry_run_diff: None,
            broken_pipe: false,
            exit_override: None,
            stdout: Box::new(io::stdout()),
            stderr: Box::new(io::stderr()),
        }
    }

    /// Creates an [`Output`] with custom writers (primarily for tests, with colour disabled).
    #[must_use]
    pub fn with_writers(json: bool, stdout: Box<dyn Write>, stderr: Box<dyn Write>) -> Self {
        Self::with_writers_color(json, false, false, stdout, stderr)
    }

    /// Creates an [`Output`] with custom writers and explicit colour flags (for tests).
    #[must_use]
    pub fn with_writers_color(
        json: bool,
        stdout_color: bool,
        stderr_color: bool,
        stdout: Box<dyn Write>,
        stderr: Box<dyn Write>,
    ) -> Self {
        Self {
            json,
            stdout_color: !json && stdout_color,
            stderr_color,
            warnings: Vec::new(),
            dry_run_diff: None,
            broken_pipe: false,
            exit_override: None,
            stdout,
            stderr,
        }
    }

    /// Asks for a non-zero exit code after a successful run, for read commands that print their
    /// findings as a normal result (for example `todo align` exits 1 when it has findings).
    pub fn set_exit(&mut self, code: ExitCode) {
        self.exit_override = Some(code);
    }

    /// The exit code requested with [`Self::set_exit`], if any.
    #[must_use]
    pub const fn requested_exit(&self) -> Option<ExitCode> {
        self.exit_override
    }

    /// Returns `true` if ANSI colour output is enabled on stdout (`R-OUT-9`).
    #[must_use]
    pub const fn use_color(&self) -> bool {
        self.stdout_color && !self.json
    }

    /// Wraps `text` in an ANSI SGR sequence when stdout colour is enabled.
    #[must_use]
    pub fn paint(&self, ansi_code: &str, text: &str) -> String {
        if self.use_color() {
            format!("\x1b[{ansi_code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in bold when stdout colour is enabled.
    #[must_use]
    pub fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }

    /// Formats `text` in dim style when stdout colour is enabled.
    #[must_use]
    pub fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }

    /// Formats `text` in red when stdout colour is enabled.
    #[must_use]
    pub fn red(&self, text: &str) -> String {
        self.paint("31", text)
    }

    /// Formats `text` in green when stdout colour is enabled.
    #[must_use]
    pub fn green(&self, text: &str) -> String {
        self.paint("32", text)
    }

    /// Formats `text` in yellow when stdout colour is enabled.
    #[must_use]
    pub fn yellow(&self, text: &str) -> String {
        self.paint("33", text)
    }

    /// Formats `text` in cyan when stdout colour is enabled.
    #[must_use]
    pub fn cyan(&self, text: &str) -> String {
        self.paint("36", text)
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
            let line = if self.stderr_color {
                format!("\x1b[33mwarning:\x1b[0m {msg}")
            } else {
                format!("warning: {msg}")
            };
            self.write_stderr_line(&line);
        }
    }

    /// Writes a plain status line to stderr in text mode (nothing in `--json` mode).
    ///
    /// For commands whose standard output is data that may be piped, such as `todo import`.
    pub fn info(&mut self, message: &str) {
        if !self.json {
            self.write_stderr_line(message);
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
            let line = if self.stderr_color {
                format!("\x1b[31merror:\x1b[0m {err}")
            } else {
                format!("error: {err}")
            };
            self.write_stderr_line(&line);
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

    #[test]
    fn color_only_on_terminal_and_without_no_color() {
        assert!(should_use_color(true, false));
        assert!(!should_use_color(false, false));
        assert!(!should_use_color(true, true));
        assert!(!should_use_color(false, true));

        let out_buf = SharedBuf::default();
        let err_buf = SharedBuf::default();
        let mut colored = Output::with_writers_color(
            false,
            true,
            true,
            Box::new(out_buf.clone()),
            Box::new(err_buf.clone()),
        );
        assert!(colored.use_color());
        assert_eq!(colored.green("ok"), "\x1b[32mok\x1b[0m");
        colored.warn("heads up");
        colored.emit_error("check", &TodoError::new(ErrorCode::BadValue, "bad"));
        let err_str = String::from_utf8(err_buf.0.borrow().clone()).unwrap();
        assert!(err_str.contains("\x1b[33mwarning:\x1b[0m heads up"));
        assert!(err_str.contains("\x1b[31merror:\x1b[0m bad"));

        let json_colored =
            Output::with_writers_color(true, true, true, Box::new(out_buf), Box::new(err_buf));
        assert!(!json_colored.use_color());
        assert_eq!(json_colored.green("ok"), "ok");
    }
}
