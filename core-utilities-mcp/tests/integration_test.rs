//! End-to-end tests that spawn the compiled `core-utilities-mcp` binary and
//! drive it over real stdin/stdout pipes, exactly as an MCP client would.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

struct TestServer {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

impl TestServer {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_core-utilities-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn core-utilities-mcp");
        let stdout = BufReader::new(child.stdout.take().expect("stdout was not piped"));
        Self { child, stdout }
    }

    fn write_raw(&mut self, raw: &str) {
        let stdin = self.child.stdin.as_mut().expect("stdin was not piped");
        writeln!(stdin, "{raw}").expect("failed to write request line");
    }

    fn read_raw(&mut self) -> String {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("failed to read response line");
        line
    }

    fn notify(&mut self, request: &Value) {
        self.write_raw(&request.to_string());
    }

    fn call(&mut self, request: &Value) -> Value {
        self.notify(request);
        serde_json::from_str(&self.read_raw()).expect("response was not valid JSON")
    }

    /// Closes stdin (simulating the client disconnecting) and polls for the
    /// process to exit, failing the test if it does not within `timeout`.
    fn close_stdin_and_wait(&mut self, timeout: Duration) -> ExitStatus {
        drop(self.child.stdin.take());
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("failed to poll child status") {
                return status;
            }
            if start.elapsed() > timeout {
                let _ = self.child.kill();
                panic!("server did not exit within {timeout:?} after stdin closed");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn initialize_and_tools_list_report_fifteen_tools() {
    let mut server = TestServer::spawn();

    let init = server.call(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}));
    assert_eq!(init["result"]["serverInfo"]["name"], "core-utilities-mcp");

    let list = server.call(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 15);
}

#[test]
fn tools_call_success_returns_structured_content() {
    let mut server = TestServer::spawn();

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "get_system_info", "arguments": {}}
    }));

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    let ctx: Value = serde_json::from_str(text).expect("tool output was not JSON");
    assert!(ctx["os"].is_string());
}

#[test]
fn tools_call_write_file_creates_file() {
    let mut server = TestServer::spawn();
    let path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-test-{}.txt",
        std::process::id()
    ));
    let path_str = path.to_str().expect("temp path was not valid UTF-8");

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "write_file", "arguments": {"path": path_str, "content": "hello from integration test"}}
    }));

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    let result: Value = serde_json::from_str(text).expect("tool output was not JSON");
    assert_eq!(result["status"], "success");
    assert_eq!(
        std::fs::read_to_string(&path).expect("written file should be readable"),
        "hello from integration test"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn tools_call_read_file_annotates_line_numbers() {
    let mut server = TestServer::spawn();
    let path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-read-test-{}.txt",
        std::process::id()
    ));
    std::fs::write(&path, "line 1\nline 2").expect("setup failed");
    let path_str = path.to_str().unwrap();

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "read_file", "arguments": {"path": path_str}}
    }));

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    let ctx: Value = serde_json::from_str(text).expect("tool output was not JSON");
    assert_eq!(ctx["status"], "success");
    assert_eq!(ctx["content"], "1\tline 1\n2\tline 2\n");

    std::fs::remove_file(&path).ok();
}

#[test]
fn tools_call_edit_file_applies_chunk_and_reports_line_delta() {
    let mut server = TestServer::spawn();
    let path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-edit-test-{}.txt",
        std::process::id()
    ));
    std::fs::write(&path, "line 1\nline 2\nline 3").expect("setup failed");
    let path_str = path.to_str().unwrap();

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "edit_file",
            "arguments": {
                "path": path_str,
                "edits": [{
                    "start_line": 2,
                    "end_line": 2,
                    "target_content": "line 2",
                    "replacement_content": "a\nb"
                }]
            }
        }
    }));

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    let result: Value = serde_json::from_str(text).expect("tool output was not JSON");
    assert_eq!(result["status"], "success");
    assert_eq!(result["line_delta"], 1);
    assert_eq!(
        std::fs::read_to_string(&path).expect("edited file should be readable"),
        "line 1\na\nb\nline 3"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn tools_call_query_data_by_path_reads_toml_and_yaml() {
    let mut server = TestServer::spawn();

    let toml_path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-query-test-{}.toml",
        std::process::id()
    ));
    std::fs::write(&toml_path, "[package]\nname = \"demo\"\n").expect("setup failed");

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_data_by_path",
            "arguments": {"path": toml_path.to_str().unwrap(), "data_path": "package.name"}
        }
    }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    assert_eq!(text, "\"demo\"");
    std::fs::remove_file(&toml_path).ok();

    let yaml_path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-query-test-{}.yaml",
        std::process::id()
    ));
    std::fs::write(&yaml_path, "service:\n  image: nginx\n").expect("setup failed");

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "query_data_by_path",
            "arguments": {"path": yaml_path.to_str().unwrap(), "data_path": "service.image"}
        }
    }));
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text field");
    assert_eq!(text, "\"nginx\"");
    std::fs::remove_file(&yaml_path).ok();
}

#[test]
fn tools_call_failure_reports_error_type() {
    let mut server = TestServer::spawn();

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "get_file_metadata", "arguments": {"path": "/nonexistent/xyz"}}
    }));

    assert_eq!(resp["result"]["isError"], true);
    assert_eq!(resp["result"]["content"][0]["_meta"]["error_type"], "File");
}

#[test]
fn malformed_json_returns_parse_error() {
    let mut server = TestServer::spawn();

    server.write_raw("not valid json");
    let resp: Value = serde_json::from_str(&server.read_raw()).expect("response was not JSON");

    assert_eq!(resp["error"]["code"], -32700);
}

#[test]
fn unknown_method_returns_method_not_found() {
    let mut server = TestServer::spawn();

    let resp = server.call(&json!({"jsonrpc": "2.0", "id": 1, "method": "totally/bogus"}));

    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn notification_produces_no_output_line() {
    let mut server = TestServer::spawn();

    // A notification (no `id`) must not produce a response line; if it did,
    // the next `call`'s read would consume that stray line instead and this
    // assertion on the *real* request's id would fail.
    server.notify(&json!({"jsonrpc": "2.0", "method": "initialized"}));
    let resp = server.call(&json!({"jsonrpc": "2.0", "id": 42, "method": "tools/list"}));

    assert_eq!(resp["id"], 42);
}

#[test]
fn stdin_close_triggers_graceful_exit() {
    let mut server = TestServer::spawn();

    let status = server.close_stdin_and_wait(Duration::from_secs(5));

    assert!(status.success());
}

#[test]
fn tools_call_write_file_refuses_overwrite_without_flag() {
    let mut server = TestServer::spawn();
    let path = std::env::temp_dir().join(format!(
        "core-utilities-mcp-test-overwrite-{}.txt",
        std::process::id()
    ));
    let path_str = path.to_str().expect("temp path was not valid UTF-8");
    std::fs::write(&path, "initial content").expect("failed to seed test file");

    let resp = server.call(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "write_file", "arguments": {"path": path_str, "content": "new content"}}
    }));

    assert_eq!(resp["result"]["isError"], true);
    assert_eq!(resp["result"]["content"][0]["_meta"]["error_type"], "File");

    std::fs::remove_file(&path).ok();
}
