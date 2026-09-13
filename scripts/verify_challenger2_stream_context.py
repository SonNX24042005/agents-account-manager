#!/usr/bin/env python3
"""
Bộ kiểm chứng thực nghiệm độc lập (Challenger 2):
1. Bảo toàn ngữ cảnh hội thoại đa lượt (`contents` array, `generationConfig`, unicode tiếng Việt, code snippets, function calls)
   khi chuyển đổi Bearer token giữa 2 tài khoản Google khác nhau.
2. Kiểm tra tính toàn vẹn của luồng SSE:
   - Không bị nhân đôi text / chunk (no duplication on first_stream.chain(stream)).
   - Đúng thứ tự từng chunk từ 0 đến N.
   - Không rò rỉ chunk lỗi RESOURCE_EXHAUSTED ra client.
   - Không nhận diện sai (false positive) khi nội dung hợp lệ có chứa từ 'quota' hoặc 'exhausted'.
3. Thử nghiệm xoay vòng đa chặng (Acc 1 [429] -> Acc 2 [Early SSE Error] -> Acc 3 [Valid Stream]).
"""

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


def log_test(name):
    print(f"\n{BOLD}{CYAN}[CHALLENGER 2 EXPERIMENT] {name}{RESET}")


def log_pass(msg):
    print(f"  {GREEN}✓ PASS:{RESET} {msg}")


def log_fail(msg):
    print(f"  {RED}✗ FAIL:{RESET} {msg}")


def log_info(msg):
    print(f"  {YELLOW}• INFO:{RESET} {msg}")


def get_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class AdversarialMockUpstreamHandler(http.server.BaseHTTPRequestHandler):
    recorded_requests = []
    mode = "default"
    custom_responses = {}

    def do_POST(self):
        self._handle()

    def do_GET(self):
        self._handle()

    def _handle(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body_bytes = self.rfile.read(content_length) if content_length > 0 else b""

        auth = self.headers.get("Authorization", "")
        token = auth.replace("Bearer ", "").strip() if "Bearer " in auth else None

        record = {
            "path": self.path,
            "method": self.command,
            "token": token,
            "body": body_bytes,
            "headers": dict(self.headers),
        }
        AdversarialMockUpstreamHandler.recorded_requests.append(record)

        mode = AdversarialMockUpstreamHandler.mode

        if mode == "rich_multiturn_context":
            # Acc 1 returns 429, Acc 2 returns stream
            if token == "token-acc-1":
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({
                    "error": {
                        "code": 429,
                        "status": "RESOURCE_EXHAUSTED",
                        "message": "Quota limit reached for token-acc-1"
                    }
                }).encode("utf-8"))
            elif token == "token-acc-2":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                self.wfile.write(b'data: {"candidates": [{"content": {"parts": [{"text": "Tra loi thanh cong tu token-acc-2"}]}}]}\n\n')
            else:
                self.send_response(401)
                self.end_headers()

        elif mode == "sse_duplication_check":
            # Acc 1 returns 429, Acc 2 returns 5 sequential SSE chunks
            if token == "token-acc-1":
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"error":{"code":429,"status":"RESOURCE_EXHAUSTED"}}')
            elif token == "token-acc-2":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                for i in range(5):
                    chunk = f'data: {{"candidates": [{{"content": {{"parts": [{{"text": "CHUNK_{i:02d}_"}}]}}}}]}}\n\n'.encode("utf-8")
                    self.wfile.write(chunk)
                    self.wfile.flush()
                    time.sleep(0.01)
            else:
                self.send_response(401)
                self.end_headers()

        elif mode == "early_error_leakage_check":
            # Acc 1 returns 200 OK with early RESOURCE_EXHAUSTED chunk
            # Acc 2 returns 200 OK with clean payload
            if token == "token-acc-1":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                err_chunk = b'data: {"error": {"code": 429, "status": "RESOURCE_EXHAUSTED", "message": "Acc 1 quota depleted in chunk 0"}}\n\n'
                self.wfile.write(err_chunk)
                self.wfile.flush()
            elif token == "token-acc-2":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                clean_chunk = b'data: {"candidates": [{"content": {"parts": [{"text": "Du lieu sach tu tai khoan 2"}]}}]}\n\n'
                self.wfile.write(clean_chunk)
                self.wfile.flush()
            else:
                self.send_response(401)
                self.end_headers()

        elif mode == "false_positive_quota_keyword_check":
            # Acc 1 returns valid response mentioning 'quota' in assistant text!
            # It MUST NOT be considered an error!
            if token == "token-acc-1":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                valid_chunk_with_keyword = b'data: {"candidates": [{"content": {"parts": [{"text": "Hien tai han ngach quota con 80% va chua bi exhausted."}]}}]}\n\n'
                self.wfile.write(valid_chunk_with_keyword)
                self.wfile.flush()
            else:
                self.send_response(401)
                self.end_headers()

        elif mode == "multi_hop_rotation":
            # Acc 1 returns 429
            # Acc 2 returns 200 with early RESOURCE_EXHAUSTED chunk
            # Acc 3 returns 200 with valid stream
            if token == "token-acc-1":
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"error":{"code":429,"status":"RESOURCE_EXHAUSTED"}}')
            elif token == "token-acc-2":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                self.wfile.write(b'data: {"error":{"code":429,"status":"RESOURCE_EXHAUSTED","message":"Acc 2 exhausted early"}}\n\n')
                self.wfile.flush()
            elif token == "token-acc-3":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                self.wfile.write(b'data: {"candidates":[{"content":{"parts":[{"text":"Thanh cong tu tai khoan 3 sau 2 lan xoay vong!"}]}}]}\n\n')
                self.wfile.flush()
            else:
                self.send_response(401)
                self.end_headers()

        else:
            self.send_response(200)
            self.end_headers()

    def log_message(self, format, *args):
        return


def create_acc(accounts_dir, email, token, quota_pct, is_active):
    acc_id = str(uuid.uuid4())
    now_iso = datetime.now(timezone.utc).isoformat()
    expires_iso = (datetime.now(timezone.utc) + timedelta(hours=2)).isoformat()
    data = {
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
    with open(os.path.join(accounts_dir, f"{acc_id}.json"), "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    return acc_id


def run_challenger_tests():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    binary_path = os.path.join(repo_root, "agent-relay", "target", "debug", "agent-relay")
    if not os.path.exists(binary_path):
        log_info("Biên dịch binary agent-relay...")
        subprocess.check_call(
            ["cargo", "build", "--manifest-path", os.path.join(repo_root, "agent-relay", "Cargo.toml")]
        )

    # Start mock upstream
    mock_port = get_free_port()
    mock_server = http.server.ThreadingHTTPServer(("127.0.0.1", mock_port), AdversarialMockUpstreamHandler)
    mock_thread = threading.Thread(target=mock_server.serve_forever, daemon=True)
    mock_thread.start()
    log_info(f"Mock upstream server lắng nghe tại port {mock_port}")

    # Temporary isolated state
    temp_dir = tempfile.TemporaryDirectory(prefix="challenger2-")
    data_dir = temp_dir.name
    accounts_dir = os.path.join(data_dir, "accounts")
    os.makedirs(accounts_dir, exist_ok=True)

    master_key = "sk-challenger2-masterkey-xyz"
    with open(os.path.join(data_dir, "master.key"), "w") as f:
        f.write(master_key)

    # Create 3 accounts
    acc1_id = create_acc(accounts_dir, "acc1@google.com", "token-acc-1", 95.0, True)
    acc2_id = create_acc(accounts_dir, "acc2@google.com", "token-acc-2", 85.0, False)
    acc3_id = create_acc(accounts_dir, "acc3@google.com", "token-acc-3", 75.0, False)
    log_info("Đã tạo 3 tài khoản trong pool: acc1 (95%), acc2 (85%), acc3 (75%)")

    # Start agent-relay
    relay_port = get_free_port()
    env = os.environ.copy()
    env["AGENT_DATA_DIR"] = data_dir
    env["AGENT_PORT"] = str(relay_port)
    env["AGENT_MASTER_KEY"] = master_key
    env["AGENT_UPSTREAM_URL"] = f"http://127.0.0.1:{mock_port}"
    env["RUST_LOG"] = "warn,agent_relay=info"

    relay_proc = subprocess.Popen([binary_path, "run"], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    try:
        # Wait for health
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
            log_fail("agent-relay không khởi động thành công!")
            return 1
        log_pass(f"agent-relay khởi động thành công tại port {relay_port}")

        reset_url = f"http://127.0.0.1:{relay_port}/api/accounts/reset"
        def reset_pool():
            req = urllib.request.Request(
                reset_url,
                data=b"{}",
                headers={"Authorization": f"Bearer {master_key}", "Content-Type": "application/json"},
                method="POST"
            )
            with urllib.request.urlopen(req, timeout=5):
                pass

        # ---------------------------------------------------------------------
        # TEST 1: Complex Multi-turn Conversation Context Preservation Across Token Rotation
        # ---------------------------------------------------------------------
        log_test("Kiểm chứng bảo toàn ngữ cảnh hội thoại đa lượt phức tạp và Bearer token")
        reset_pool()
        AdversarialMockUpstreamHandler.mode = "rich_multiturn_context"
        AdversarialMockUpstreamHandler.recorded_requests.clear()

        # Build a 10-turn rich conversation with code, vietnamese text, function call & response
        multiturn_contents = [
            {"role": "user", "parts": [{"text": "Xin chào! Bạn hãy viết hàm tính giai thừa bằng Rust."}]},
            {"role": "model", "parts": [{"text": "Chào bạn! Đây là hàm giai thừa:\n```rust\nfn factorial(n: u64) -> u64 {\n    (1..=n).product()\n}\n```"}]},
            {"role": "user", "parts": [{"text": "Bây giờ hãy đọc tệp Cargo.toml trong thư mục dự án."}]},
            {"role": "model", "parts": [
                {"text": "Tôi sẽ gọi công cụ view_file để đọc Cargo.toml."},
                {"functionCall": {"name": "view_file", "args": {"path": "Cargo.toml", "encoding": "utf-8"}}}
            ]},
            {"role": "user", "parts": [
                {"functionResponse": {"name": "view_file", "response": {"status": "success", "content": "[package]\nname = \"agent-relay\"\nversion = \"0.1.0\""}}}
            ]},
            {"role": "model", "parts": [{"text": "Tôi đã đọc xong tệp Cargo.toml. Phiên bản hiện tại là 0.1.0."}]},
            {"role": "user", "parts": [{"text": "Hãy phân tích độ phức tạp thời gian O(n) và không gian O(1) của thuật toán."}]},
            {"role": "model", "parts": [{"text": "Thuật toán có độ phức tạp thời gian là O(n) và không gian bộ nhớ O(1)."}]},
            {"role": "user", "parts": [{"text": "Ký tự đặc biệt: <tag>test & verify \"double quotes\" and 'single'</tag> 🚀 🇻🇳"}]},
            {"role": "user", "parts": [{"text": "Lượt thứ 10: Tiếp tục hoàn thiện module kiểm thử!"}]}
        ]

        complex_payload = {
            "contents": multiturn_contents,
            "systemInstruction": {
                "parts": [{"text": "Bạn là kỹ sư phần mềm cao cấp phụ trách đánh giá mã nguồn."}]
            },
            "generationConfig": {
                "temperature": 0.2,
                "topP": 0.95,
                "topK": 40,
                "candidateCount": 1,
                "maxOutputTokens": 4096,
                "stopSequences": ["<STOP>", "<END>"]
            },
            "safetySettings": [
                {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"}
            ]
        }

        payload_bytes = json.dumps(complex_payload, ensure_ascii=False).encode("utf-8")
        passthrough_url = f"http://127.0.0.1:{relay_port}/v1internal:streamGenerateContent?alt=sse"

        req = urllib.request.Request(
            passthrough_url,
            data=payload_bytes,
            headers={"Content-Type": "application/json; charset=utf-8"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            client_received = resp.read().decode("utf-8")

        assert "Tra loi thanh cong tu token-acc-2" in client_received
        assert len(AdversarialMockUpstreamHandler.recorded_requests) == 2, f"Kỳ vọng 2 requests upstream, nhận được {len(AdversarialMockUpstreamHandler.recorded_requests)}"

        req1 = AdversarialMockUpstreamHandler.recorded_requests[0]
        req2 = AdversarialMockUpstreamHandler.recorded_requests[1]

        # Check Bearer token swap
        assert req1["token"] == "token-acc-1", f"Request 1 phải mang token-acc-1, nhưng nhận {req1['token']}"
        assert req2["token"] == "token-acc-2", f"Request 2 phải mang token-acc-2, nhưng nhận {req2['token']}"
        log_pass("Bearer token đã được hoán đổi chính xác từ token-acc-1 sang token-acc-2.")

        # Check exact byte match & JSON deep equality of request payload
        assert req1["body"] == req2["body"], "Payload bytes giữa request 1 và request 2 không khớp byte-for-byte!"
        assert req2["body"] == payload_bytes, "Payload bytes gửi đến upstream tài khoản 2 không khớp với dữ liệu gốc của client!"

        req2_json = json.loads(req2["body"].decode("utf-8"))
        assert req2_json["contents"] == complex_payload["contents"], "Mảng contents đa lượt bị sai lệch!"
        assert req2_json["systemInstruction"] == complex_payload["systemInstruction"], "systemInstruction bị sai lệch!"
        assert req2_json["generationConfig"] == complex_payload["generationConfig"], "generationConfig bị sai lệch!"
        assert req2_json["safetySettings"] == complex_payload["safetySettings"], "safetySettings bị sai lệch!"
        log_pass("Toàn bộ 10 lượt hội thoại (multiturn contents), tiếng Việt có dấu, code blocks, functionCall/Response và systemInstruction được bảo toàn 100% nguyên vẹn.")

        # ---------------------------------------------------------------------
        # TEST 2: SSE Stream Integrity & No Text Duplication Check
        # ---------------------------------------------------------------------
        log_test("Kiểm tra tính toàn vẹn của luồng SSE và hiện tượng nhân đôi text (Duplication Check)")
        reset_pool()
        AdversarialMockUpstreamHandler.mode = "sse_duplication_check"
        AdversarialMockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            passthrough_url,
            data=b'{"contents": [{"role": "user", "parts": [{"text": "SSE streaming test"}]}]}',
            headers={"Content-Type": "application/json"},
            method="POST"
        )

        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            # Read streaming response incrementally
            raw_sse_bytes = resp.read()
            raw_sse_text = raw_sse_bytes.decode("utf-8")

        # Parse SSE text chunks
        expected_tokens = ["CHUNK_00_", "CHUNK_01_", "CHUNK_02_", "CHUNK_03_", "CHUNK_04_"]
        for tok in expected_tokens:
            occurrences = raw_sse_text.count(tok)
            assert occurrences == 1, f"Token {tok} xuất hiện {occurrences} lần (kỳ vọng đúng 1 lần duy nhất, không nhân đôi hay mất mát)!"

        # Verify order of chunks
        last_idx = -1
        for tok in expected_tokens:
            curr_idx = raw_sse_text.find(tok)
            assert curr_idx > last_idx, f"Thứ tự chunk không bảo toàn: {tok} xuất hiện ở vị trí {curr_idx} trước {last_idx}"
            last_idx = curr_idx

        log_pass("Tất cả 5 chunk SSE ('CHUNK_00_' đến 'CHUNK_04_') đến client theo đúng thứ tự tuần tự tuyệt đối.")
        log_pass("Xác nhận: Chunk 0 KHÔNG bị lặp lại (zero duplication) qua cơ chế first_stream.chain(stream).")

        # ---------------------------------------------------------------------
        # TEST 3: Early Quota Error in SSE Frame Leakage Check
        # ---------------------------------------------------------------------
        log_test("Kiểm tra rò rỉ chunk lỗi RESOURCE_EXHAUSTED ra client")
        reset_pool()
        AdversarialMockUpstreamHandler.mode = "early_error_leakage_check"
        AdversarialMockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            passthrough_url,
            data=b'{"contents": [{"role": "user", "parts": [{"text": "Leakage test"}]}]}',
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            client_body = resp.read().decode("utf-8")

        assert "Du lieu sach tu tai khoan 2" in client_body
        assert "RESOURCE_EXHAUSTED" not in client_body, "Lỗi RESOURCE_EXHAUSTED bị rò rỉ ra client!"
        assert "Acc 1 quota depleted" not in client_body, "Nội dung lỗi của tài khoản 1 bị lọt vào phản hồi client!"
        assert "error" not in client_body.lower() or "\"error\"" not in client_body, "JSON error bị rò rỉ trong luồng SSE!"
        log_pass("First-chunk response gate chặn hoàn toàn frame lỗi từ tài khoản 1; client nhận 100% dữ liệu sạch từ tài khoản 2.")

        # ---------------------------------------------------------------------
        # TEST 4: False Positive Resistance (Valid Content with 'quota' keyword)
        # ---------------------------------------------------------------------
        log_test("Kiểm tra khả năng chống nhận diện sai (False Positive) khi văn bản hợp lệ chứa từ khóa 'quota'/'exhausted'")
        reset_pool()
        AdversarialMockUpstreamHandler.mode = "false_positive_quota_keyword_check"
        AdversarialMockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            passthrough_url,
            data=b'{"contents": [{"role": "user", "parts": [{"text": "Hoi ve quota"}]}]}',
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            client_body = resp.read().decode("utf-8")

        assert "Hien tai han ngach quota con 80% va chua bi exhausted." in client_body
        assert len(AdversarialMockUpstreamHandler.recorded_requests) == 1, f"Kỳ vọng đúng 1 request (không xoay vòng vô cớ), nhưng ghi nhận {len(AdversarialMockUpstreamHandler.recorded_requests)}"
        assert AdversarialMockUpstreamHandler.recorded_requests[0]["token"] == "token-acc-1"
        log_pass("Relay không nhận diện sai (no false positive) khi text của mô hình nhắc đến 'quota' và 'exhausted'. Phản hồi được stream tức thì từ tài khoản 1.")

        # ---------------------------------------------------------------------
        # TEST 5: Multi-hop Rotation (Acc 1 [429] -> Acc 2 [Early SSE Error] -> Acc 3 [Valid Stream])
        # ---------------------------------------------------------------------
        log_test("Kiểm tra xoay vòng đa chặng (Multi-hop: Acc 1 -> Acc 2 -> Acc 3)")
        reset_pool()
        AdversarialMockUpstreamHandler.mode = "multi_hop_rotation"
        AdversarialMockUpstreamHandler.recorded_requests.clear()

        req = urllib.request.Request(
            passthrough_url,
            data=b'{"contents": [{"role": "user", "parts": [{"text": "Multi-hop rotation test"}]}]}',
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            client_body = resp.read().decode("utf-8")

        assert "Thanh cong tu tai khoan 3 sau 2 lan xoay vong!" in client_body
        assert len(AdversarialMockUpstreamHandler.recorded_requests) == 3, f"Kỳ vọng 3 requests trong chuỗi xoay vòng, nhận được {len(AdversarialMockUpstreamHandler.recorded_requests)}"
        assert AdversarialMockUpstreamHandler.recorded_requests[0]["token"] == "token-acc-1"
        assert AdversarialMockUpstreamHandler.recorded_requests[1]["token"] == "token-acc-2"
        assert AdversarialMockUpstreamHandler.recorded_requests[2]["token"] == "token-acc-3"
        log_pass("Relay đã tự động chuyển qua 2 lần lỗi liên tiếp (Acc 1 429 và Acc 2 First-Chunk Gate) đến Acc 3 thành công rực rỡ.")

        print(f"\n{BOLD}{GREEN}=== TẤT CẢ CÁC BÀI KIỂM CHỨNG CỦA CHALLENGER 2 ĐỀU ĐẠT CHUẨN XUẤT SẮC ==={RESET}\n")
        return 0

    finally:
        log_info("Dọn dẹp tiến trình relay và mock server...")
        relay_proc.terminate()
        try:
            relay_proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            relay_proc.kill()
        mock_server.shutdown()
        temp_dir.cleanup()
        log_info("Dọn dẹp hoàn tất.")


if __name__ == "__main__":
    sys.exit(run_challenger_tests())
