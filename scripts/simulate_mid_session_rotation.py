#!/usr/bin/env python3
"""
Mô phỏng kiểm thử tích hợp xoay vòng tài khoản giữa phiên (mid-session rotation) cho agent-relay.
Kịch bản mô phỏng tương tác thực tế giữa agy CLI và daemon agent-relay:
1. Kịch bản 1: Upstream trả về 429 tức thì cho tài khoản 1 -> relay tự xoay sang tài khoản 2.
2. Kịch bản 2: First-chunk response gate chặn lỗi RESOURCE_EXHAUSTED sớm trong SSE -> xoay sang tài khoản 2.
3. Kịch bản 3: Bảo toàn ngữ cảnh hội thoại đa lượt và lệnh gọi công cụ sang tài khoản thứ hai.
4. Kịch bản 4: Toàn bộ pool tài khoản cạn hạn ngạch -> trả 429 an toàn, không treo kết nối.
5. Kịch bản 5: Xác thực loopback miễn trừ API key cho /v1internal/* nhưng bảo vệ /api/*.
"""

import argparse
import http.server
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from datetime import datetime, timezone, timedelta

GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
RESET = "\033[0m"


def log_step(title):
    print(f"\n{BOLD}{CYAN}=== {title} ==={RESET}")


def log_success(msg):
    print(f"  {GREEN}✓{RESET} {msg}")


def log_fail(msg):
    print(f"  {RED}✗{RESET} {msg}")


def log_info(msg):
    print(f"  {YELLOW}•{RESET} {msg}")


class MockUpstreamHandler(http.server.BaseHTTPRequestHandler):
    recorded_requests = []
    mode = "immediate_429"
    acc1_token = "sim-token-1"
    acc2_token = "sim-token-2"

    def do_POST(self):
        self._handle_request()

    def do_GET(self):
        self._handle_request()

    def _handle_request(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body_bytes = self.rfile.read(content_length) if content_length > 0 else b""

        auth = self.headers.get("Authorization", "")
        token = auth.replace("Bearer ", "").strip() if "Bearer " in auth else None

        req_record = {
            "path": self.path,
            "method": self.command,
            "token": token,
            "body": body_bytes,
            "headers": dict(self.headers),
        }
        MockUpstreamHandler.recorded_requests.append(req_record)

        if MockUpstreamHandler.mode == "immediate_429":
            if token == MockUpstreamHandler.acc1_token:
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                err_payload = json.dumps({
                    "error": {
                        "code": 429,
                        "status": "RESOURCE_EXHAUSTED",
                        "message": "Resource has been exhausted (e.g. check quota)."
                    }
                }).encode("utf-8")
                self.wfile.write(err_payload)
            elif token == MockUpstreamHandler.acc2_token:
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                sse = b'data: {"candidates": [{"content": {"parts": [{"text": "Phan hoi tu tai khoan 2"}]}}]}\n\n'
                self.wfile.write(sse)
            else:
                self.send_response(401)
                self.end_headers()

        elif MockUpstreamHandler.mode == "early_chunk_error":
            if token == MockUpstreamHandler.acc1_token:
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                chunk1 = b'data: {"error": {"code": 429, "status": "RESOURCE_EXHAUSTED", "message": "Can quota som"}}\n\n'
                self.wfile.write(chunk1)
                self.wfile.flush()
            elif token == MockUpstreamHandler.acc2_token:
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                chunk1 = b'data: {"candidates": [{"content": {"parts": [{"text": "Du lieu hop le 1"}]}}]}\n\n'
                chunk2 = b'data: {"candidates": [{"content": {"parts": [{"text": " va du lieu 2"}]}}]}\n\n'
                self.wfile.write(chunk1)
                self.wfile.flush()
                self.wfile.write(chunk2)
                self.wfile.flush()
            else:
                self.send_response(401)
                self.end_headers()

        elif MockUpstreamHandler.mode == "rich_context":
            if token == MockUpstreamHandler.acc1_token:
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"error": {"code": 429, "status": "RESOURCE_EXHAUSTED"}}')
            elif token == MockUpstreamHandler.acc2_token:
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                self.wfile.write(b'data: {"candidates": [{"content": {"parts": [{"text": "Ngu canh bao toan"}]}}]}\n\n')
            else:
                self.send_response(401)
                self.end_headers()

        elif MockUpstreamHandler.mode == "all_exhausted":
            self.send_response(429)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"error": {"code": 429, "message": "Tat ca tai khoan deu can quota"}}')

        elif MockUpstreamHandler.mode == "simple_success":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.end_headers()
            self.wfile.write(b'data: {"candidates": [{"content": {"parts": [{"text": "Loopback OK"}]}}]}\n\n')

    def log_message(self, format, *args):
        # Silence default HTTP server logging
        return


def get_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def create_account_file(accounts_dir, email, token, quota_pct, is_active):
    acc_id = str(uuid.uuid4())
    now_iso = datetime.now(timezone.utc).isoformat()
    expires_iso = (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat()
    acc_data = {
        "id": acc_id,
        "email": email,
        "access_token": token,
        "refresh_token": f"ref-{acc_id}",
        "expires_at": expires_iso,
        "custom_label": None,
        "device_profile": {
            "machine_id": uuid.uuid4().hex,
            "mac_machine_id": uuid.uuid4().hex,
            "dev_device_id": str(uuid.uuid4()),
            "sqm_id": f"{{{str(uuid.uuid4()).upper()}}}",
        },
        "quota_percentage": quota_pct,
        "quota_checked_at": now_iso,
        "quota_groups": [],
        "is_active": is_active,
        "rate_limit_until": None,
        "last_warmup_at": None,
    }
    file_path = os.path.join(accounts_dir, f"{acc_id}.json")
    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(acc_data, f, indent=2)
    return acc_id


def run_simulation():
    parser = argparse.ArgumentParser(description="Mô phỏng xoay vòng tài khoản giữa phiên cho agent-relay")
    parser.add_argument("--binary", help="Đường dẫn tới binary agent-relay")
    args = parser.parse_args()

    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    binary_path = args.binary
    if not binary_path:
        candidate = os.path.join(repo_root, "agent-relay", "target", "debug", "agent-relay")
        if not os.path.exists(candidate):
            log_info("Chưa tìm thấy binary agent-relay, đang biên dịch...")
            subprocess.check_call(
                ["cargo", "build", "--manifest-path", os.path.join(repo_root, "agent-relay", "Cargo.toml")]
            )
        binary_path = candidate

    log_info(f"Sử dụng binary: {binary_path}")

    # 1. Start Mock Upstream Server
    mock_port = get_free_port()
    mock_server = http.server.ThreadingHTTPServer(("127.0.0.1", mock_port), MockUpstreamHandler)
    mock_thread = threading.Thread(target=mock_server.serve_forever, daemon=True)
    mock_thread.start()
    log_info(f"Máy chủ giả lập Google Cloud Code PA lắng nghe tại: http://127.0.0.1:{mock_port}")

    # 2. Setup isolated temp directory
    temp_dir = tempfile.TemporaryDirectory(prefix="aam-sim-")
    data_dir = temp_dir.name
    accounts_dir = os.path.join(data_dir, "accounts")
    os.makedirs(accounts_dir, exist_ok=True)

    master_key = "sk-aam-simmasterkey123456789"
    with open(os.path.join(data_dir, "master.key"), "w") as f:
        f.write(master_key)

    # Populate 2 accounts
    acc1_id = create_account_file(accounts_dir, "sim-acc1@example.com", "sim-token-1", 95.0, True)
    acc2_id = create_account_file(accounts_dir, "sim-acc2@example.com", "sim-token-2", 85.0, False)
    log_info(f"Đã tạo tài khoản 1 (95% quota, active): sim-acc1@example.com")
    log_info(f"Đã tạo tài khoản 2 (85% quota, reserve): sim-acc2@example.com")

    # 3. Start agent-relay process
    relay_port = get_free_port()
    env = os.environ.copy()
    env["AGENT_DATA_DIR"] = data_dir
    env["AGENT_PORT"] = str(relay_port)
    env["AGENT_MASTER_KEY"] = master_key
    env["AGENT_UPSTREAM_URL"] = f"http://127.0.0.1:{mock_port}"
    env["RUST_LOG"] = "warn,agent_relay=info"

    relay_process = subprocess.Popen(
        [binary_path, "run"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    try:
        # Wait for relay to become healthy
        log_info(f"Đang khởi động agent-relay tại port {relay_port}...")
        healthy = False
        health_url = f"http://127.0.0.1:{relay_port}/api/health"
        for _ in range(50):
            try:
                with urllib.request.urlopen(health_url, timeout=0.5) as r:
                    if r.status == 200:
                        healthy = True
                        break
            except Exception:
                time.sleep(0.1)

        if not healthy:
            log_fail("Daemon agent-relay không khởi động kịp thời!")
            if relay_process.poll() is not None:
                stdout_data = relay_process.stdout.read().decode('utf-8', errors='replace')
                stderr_data = relay_process.stderr.read().decode('utf-8', errors='replace')
                log_info(f"Relay exit code: {relay_process.returncode}")
                log_info(f"Stdout: {stdout_data}")
                log_info(f"Stderr: {stderr_data}")
            return 1
        log_success("Daemon agent-relay đã sẵn sàng tiếp nhận yêu cầu.")

        # --- SCENARIO 1: Immediate 429 ---
        log_step("Kịch bản 1: Upstream trả về 429 tức thì cho tài khoản 1")
        MockUpstreamHandler.mode = "immediate_429"
        MockUpstreamHandler.recorded_requests.clear()

        req_data = json.dumps({"contents": [{"role": "user", "parts": [{"text": "Cau hoi kiem thu 1"}]}]}).encode("utf-8")
        req = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse",
            data=req_data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            status = resp.status
            body = resp.read().decode("utf-8")

        assert status == 200, f"Kỳ vọng 200, nhận được {status}"
        assert "Phan hoi tu tai khoan 2" in body, "Phải nhận được luồng SSE từ tài khoản 2"
        assert len(MockUpstreamHandler.recorded_requests) == 2, "Upstream phải nhận đủ 2 request xoay vòng"
        assert MockUpstreamHandler.recorded_requests[0]["token"] == "sim-token-1"
        assert MockUpstreamHandler.recorded_requests[1]["token"] == "sim-token-2"
        log_success("Relay tự động bắt mã 429, đánh dấu cooldown tài khoản 1 và xoay sang tài khoản 2 thành công.")

        # --- SCENARIO 2: First-Chunk Response Gate ---
        log_step("Kịch bản 2: Upstream trả về lỗi cạn hạn ngạch ở chunk SSE đầu tiên")
        # Reset cooldown tài khoản 1 để test scenario 2
        reset_req = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/api/accounts/reset",
            data=b"{}",
            headers={"Authorization": f"Bearer {master_key}", "Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(reset_req, timeout=5):
            pass

        MockUpstreamHandler.mode = "early_chunk_error"
        MockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse",
            data=req_data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            status = resp.status
            body = resp.read().decode("utf-8")

        assert status == 200
        assert "Du lieu hop le 1" in body and "va du lieu 2" in body
        assert "RESOURCE_EXHAUSTED" not in body
        log_success("First-chunk response gate chặn thành công lỗi cạn quota sớm, xoay sang tài khoản 2 mượt mà.")

        # --- SCENARIO 3: Context Preservation ---
        log_step("Kịch bản 3: Bảo toàn trọn vẹn ngữ cảnh hội thoại đa lượt")
        with urllib.request.urlopen(reset_req, timeout=5):
            pass

        MockUpstreamHandler.mode = "rich_context"
        MockUpstreamHandler.recorded_requests.clear()

        rich_payload = {
            "contents": [
                {"role": "user", "parts": [{"text": "Liet ke cac tep"}]},
                {
                    "role": "model",
                    "parts": [
                        {"text": "Toi se goi cong cu list_dir"},
                        {"functionCall": {"name": "list_dir", "args": {"path": "."}}},
                    ],
                },
                {
                    "role": "user",
                    "parts": [{"functionResponse": {"name": "list_dir", "response": {"files": ["main.rs"]}}}],
                },
                {"role": "user", "parts": [{"text": "Tiep tuc viet kiem thu"}]},
            ],
            "generationConfig": {"temperature": 0.1, "maxOutputTokens": 2048},
        }
        rich_data = json.dumps(rich_payload).encode("utf-8")
        req = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse",
            data=rich_data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            resp.read()

        assert len(MockUpstreamHandler.recorded_requests) == 2
        acc2_req_body = json.loads(MockUpstreamHandler.recorded_requests[1]["body"].decode("utf-8"))
        assert acc2_req_body["contents"] == rich_payload["contents"]
        assert acc2_req_body["generationConfig"] == rich_payload["generationConfig"]
        log_success("Mảng contents đa lượt và lệnh gọi functionCall/functionResponse truyền sang tài khoản 2 nguyên vẹn 100%.")

        # --- SCENARIO 4: Pool Exhaustion ---
        log_step("Kịch bản 4: Toàn bộ pool tài khoản cạn hạn ngạch")
        MockUpstreamHandler.mode = "all_exhausted"
        MockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse",
            data=req_data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                log_fail(f"Kỳ vọng 429 nhưng nhận được {resp.status}")
                return 1
        except urllib.error.HTTPError as err:
            assert err.code in (429, 503), f"Kỳ vọng HTTP 429/503, nhận được {err.code}"
            err_body = err.read().decode("utf-8")
            assert "RESOURCE_EXHAUSTED" in err_body or "quota" in err_body or "exhausted" in err_body
            log_success("Relay trả mã 429 an toàn cho client, không treo tiến trình hay bị panic khi hết sạch pool.")

        # --- SCENARIO 5: Loopback Authentication ---
        log_step("Kịch bản 5: Xác thực loopback miễn trừ API key cho /v1internal/*")
        with urllib.request.urlopen(reset_req, timeout=5):
            pass

        MockUpstreamHandler.mode = "simple_success"
        MockUpstreamHandler.recorded_requests.clear()

        # 1. Call passthrough without master key
        req_loopback = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse",
            data=b'{"contents": []}',
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req_loopback, timeout=5) as resp:
            assert resp.status == 200
        log_success("Truy vấn từ loopback vào /v1internal/* được chuyển tiếp thành công mà không cần master key.")

        # 2. Call admin API without master key
        req_admin_unauth = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/api/accounts",
            method="GET",
        )
        try:
            with urllib.request.urlopen(req_admin_unauth, timeout=5) as resp:
                log_fail("Endpoint /api/accounts phải yêu cầu master key nhưng lại cho phép truy cập trái phép!")
                return 1
        except urllib.error.HTTPError as err:
            assert err.code == 401
            log_success("Endpoint quản trị /api/* từ chối truy cập không có master key (401 Unauthorized).")

        # 3. Call admin API with master key
        req_admin_auth = urllib.request.Request(
            f"http://127.0.0.1:{relay_port}/api/accounts",
            headers={"Authorization": f"Bearer {master_key}"},
            method="GET",
        )
        with urllib.request.urlopen(req_admin_auth, timeout=5) as resp:
            assert resp.status == 200
        log_success("Endpoint quản trị /api/* chấp nhận master key hợp lệ.")

        print(f"\n{BOLD}{GREEN}=== TẤT CẢ 5 KỊCH BẢN MÔ PHỎNG ĐỀU VƯỢT QUA THÀNH CÔNG ==={RESET}\n")
        return 0

    finally:
        log_info("Đang dọn dẹp tiến trình và tệp tạm...")
        relay_process.terminate()
        try:
            relay_process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            relay_process.kill()
        mock_server.shutdown()
        temp_dir.cleanup()
        log_info("Dọn dẹp hoàn tất.")


if __name__ == "__main__":
    sys.exit(run_simulation())
