import argparse
import ctypes
from ctypes import wintypes
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time
import urllib.request


def now():
    return datetime.now(timezone.utc).isoformat()


class DiscordWindow:
    def __init__(self, port):
        from websocket import create_connection

        with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list", timeout=5) as response:
            targets = json.load(response)
        candidates = [
            target for target in targets
            if target.get("type") == "page"
            and target.get("url", "").startswith("https://discord.com/")
        ]
        if len(candidates) != 1:
            raise RuntimeError("Expected one Discord application page")
        self.socket = create_connection(
            candidates[0]["webSocketDebuggerUrl"], timeout=8, suppress_origin=True
        )
        self.sequence = 0

    def close(self):
        self.socket.close()

    def call(self, method, params=None):
        self.sequence += 1
        identity = self.sequence
        self.socket.send(json.dumps({"id": identity, "method": method, "params": params or {}}))
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            result = json.loads(self.socket.recv())
            if result.get("id") == identity:
                if "error" in result:
                    raise RuntimeError(str(result["error"]))
                return result.get("result", {})
        raise TimeoutError(f"Discord UI command timed out: {method}")

    def evaluate(self, expression, by_value=True):
        result = self.call("Runtime.evaluate", {
            "expression": expression, "returnByValue": by_value, "userGesture": True,
        })
        if "exceptionDetails" in result:
            raise RuntimeError(result["exceptionDetails"].get("text", "Discord UI script failed"))
        return result.get("result", {}).get("value") if by_value else result["result"]


def game_windows():
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
    user32.IsWindowVisible.argtypes = [wintypes.HWND]
    user32.IsIconic.argtypes = [wintypes.HWND]
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
    kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel32.OpenProcess.restype = wintypes.HANDLE
    kernel32.QueryFullProcessImageNameW.argtypes = [
        wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD),
    ]
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    windows = []

    @callback_type
    def collect(hwnd, _):
        if not user32.IsWindowVisible(hwnd) or user32.IsIconic(hwnd):
            return True
        process_id = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(process_id))
        handle = kernel32.OpenProcess(0x1000, False, process_id.value)
        if not handle:
            return True
        try:
            path = ctypes.create_unicode_buffer(32768)
            size = wintypes.DWORD(len(path))
            if not kernel32.QueryFullProcessImageNameW(handle, 0, path, ctypes.byref(size)):
                return True
            if not re.fullmatch(r"origins(?:[_-][\w-]+)?\.exe", Path(path.value).name, re.IGNORECASE):
                return True
            title = ctypes.create_unicode_buffer(2048)
            user32.GetWindowTextW(hwnd, title, len(title))
            rectangle = wintypes.RECT()
            user32.GetWindowRect(hwnd, ctypes.byref(rectangle))
            if rectangle.right - rectangle.left < 100 or rectangle.bottom - rectangle.top < 100:
                return True
            windows.append({"hwnd": int(hwnd), "pid": process_id.value, "title": title.value,
                            "executable": path.value})
        finally:
            kernel32.CloseHandle(handle)
        return True

    user32.EnumWindows(collect, 0)
    return windows


def capture_window(hwnd, output):
    from PIL import Image, ImageStat
    from windows_capture import WindowsCapture

    complete = threading.Event()
    errors = []
    if not any(window["hwnd"] == hwnd for window in game_windows()):
        raise RuntimeError("The selected window is no longer an Origins window")
    capture = WindowsCapture(cursor_capture=False, window_hwnd=hwnd)

    @capture.event
    def on_frame_arrived(frame, control):
        try:
            image = Image.frombytes("RGB", (frame.width, frame.height),
                                    frame.frame_buffer.tobytes(), "raw", "BGRX")
            sample = image.resize((64, 36)).convert("L")
            if sample.getextrema()[1] < 8 or ImageStat.Stat(sample).stddev[0] < 0.5:
                return
            image.thumbnail((2560, 1440))
            image.save(output, format="JPEG", quality=85)
        except Exception as error:
            errors.append(str(error))
        control.stop()
        complete.set()

    @capture.event
    def on_closed():
        errors.append("Window closed before capture completed")
        complete.set()

    control = capture.start_free_threaded()
    if not complete.wait(10):
        control.stop()
        raise TimeoutError("No fresh window frame; minimized windows cannot be captured")
    control.wait()
    if errors:
        raise RuntimeError("; ".join(errors))


def composer_state(ui):
    return ui.evaluate("""(() => {
        const editor = document.querySelector('[role="textbox"][contenteditable="true"]');
        return {path: location.pathname, label: editor?.getAttribute('aria-label'),
                draft: (editor?.innerText || '').replace(/[\\uFEFF\\u200B\\s]/g, ''),
                dialogs: [...document.querySelectorAll('[role="dialog"]')].map(e => e.innerText),
                uploads: [...(editor?.closest('form')?.querySelectorAll('[aria-label]') || [])].filter(
                    e => /remove attachment/i.test(e.getAttribute('aria-label'))).length};
    })()""")


def navigate(ui, config):
    target = f"/channels/{config['guild_id']}/{config['channel_id']}"
    current = composer_state(ui)
    if current["draft"] or current["dialogs"] or current["uploads"]:
        raise RuntimeError("Discord has a draft, attachment, or dialog; waiting without changing it")
    if current["path"] != target:
        guild = json.dumps(f"guildsnav___{config['guild_id']}")
        ui.evaluate(f"""(() => {{
            const server = [...document.querySelectorAll('[data-list-item-id]')].find(
                e => e.getAttribute('data-list-item-id') === {guild});
            if (!server || !/psych ward/i.test(server.innerText)) throw Error('Psych Ward server unavailable');
            server.click();
        }})()""")
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            clicked = ui.evaluate(f"""(() => {{
                const channel = [...document.querySelectorAll('a[href]')].find(
                    e => e.getAttribute('href') === {json.dumps(target)});
                if (!channel) return false;
                channel.click(); return true;
            }})()""")
            if clicked:
                break
            time.sleep(0.2)
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        state = composer_state(ui)
        if state["path"] == target and state["label"] == "Message #mod-chat":
            if state["draft"] or state["dialogs"] or state["uploads"]:
                raise RuntimeError("mod-chat has a draft, attachment, or dialog; waiting")
            return
        time.sleep(0.2)
    raise RuntimeError("Discord is not ready in Psych Ward / mod-chat")


def attachment_messages(ui):
    return ui.evaluate("""[...document.querySelectorAll('[id^="chat-messages-"]')].map(e => ({
        id: e.id,
        images: [...e.querySelectorAll('a[href],img[src]')].map(a =>
            a.getAttribute('href') || a.getAttribute('src')).filter(
                url => url && /\\/attachments\\//.test(url) && /origins-latest\\.jpg/.test(url)),
        pending: !!e.querySelector('[class*="isSending"], [class*="isFailed"]')
    })).filter(e => e.images.length && !e.pending)""")


def confirmed_messages(ui, pending, config):
    target = f"/channels/{config['guild_id']}/{config['channel_id']}"
    if ui.evaluate("location.pathname") != target:
        return []
    return [message for message in attachment_messages(ui)
            if message["id"] not in pending["baseline"]
            and int(message["id"].rsplit("-", 1)[-1]) >= pending.get("minimum_id", 0)]


def write_status(state_dir, state, **updates):
    state.update(updates, updated_at=now())
    temporary = state_dir / "status.tmp"
    temporary.write_text(json.dumps(state, indent=2), encoding="utf-8")
    temporary.replace(state_dir / "status.json")


def send_capture(ui, config, output, state_dir, state):
    navigate(ui, config)
    before = attachment_messages(ui)
    baseline = [message["id"] for message in before]
    pending = {"at": now(), "baseline": baseline, "phase": "attaching",
               "minimum_id": (int(time.time() * 1000) - 1420070400000 - 2000) << 22,
               "sha256": hashlib.sha256(output.read_bytes()).hexdigest()}
    write_status(state_dir, state, status="attaching", pending=pending)
    inputs = ui.evaluate("document.querySelectorAll('input[type=\"file\"]').length")
    if inputs != 1:
        write_status(state_dir, state, pending=None)
        raise RuntimeError("Expected one Discord attachment input")
    node = ui.evaluate("document.querySelector('input[type=\"file\"]')", by_value=False)
    ui.call("DOM.setFileInputFiles", {"files": [str(output.resolve())], "objectId": node["objectId"]})
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        ready = ui.evaluate("""(() => {
            const editor = document.querySelector('[role="textbox"][aria-label="Message #mod-chat"]');
            const attachments = editor?.closest('form')?.querySelector('[data-list-id="attachments"]');
            return attachments?.children.length === 1 && attachments.innerText.includes('origins-latest.jpg');
        })()""")
        if ready:
            break
        time.sleep(0.2)
    else:
        raise RuntimeError("Attachment preview did not appear; preserving uncertain draft")
    submit_attachment(ui, config, state_dir, state)


def submit_attachment(ui, config, state_dir, state):
    pending = state["pending"]
    target = f"/channels/{config['guild_id']}/{config['channel_id']}"
    state_before_send = composer_state(ui)
    if (state_before_send["path"] != target or state_before_send["draft"]
            or state_before_send["uploads"] != 1 or state_before_send["dialogs"]):
        raise RuntimeError("Discord changed while preparing screenshot; preserving draft")
    ui.evaluate("""(() => {
        const editor = document.querySelector('[role="textbox"][aria-label="Message #mod-chat"]');
        if (!editor) throw Error('Composer unavailable');
        editor.focus();
    })()""")
    pending["phase"] = "sending"
    write_status(state_dir, state, status="sending", pending=pending)
    ui.call("Input.dispatchKeyEvent", {"type": "keyDown", "key": "Enter", "code": "Enter",
                                      "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13})
    ui.call("Input.dispatchKeyEvent", {"type": "keyUp", "key": "Enter", "code": "Enter",
                                      "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13})
    deadline = time.monotonic() + 18
    while time.monotonic() < deadline:
        messages = confirmed_messages(ui, pending, config)
        if messages:
            write_status(state_dir, state, status="watching", pending=None,
                         sent_count=state.get("sent_count", 0) + 1, last_sent_at=now(),
                         last_message=messages[-1]["id"], error=None)
            return
        time.sleep(0.4)
    raise RuntimeError("Upload outcome uncertain; paused to prevent duplicate posts")


def run_sender(args):
    state_dir = args.state_dir.resolve()
    state_dir.mkdir(parents=True, exist_ok=True)
    config = json.loads(args.config.read_text(encoding="utf-8-sig"))
    status_path = state_dir / "status.json"
    state = json.loads(status_path.read_text(encoding="utf-8")) if status_path.exists() else {}
    state.update(worker_pid=os.getpid(), started_at=now(), interval_seconds=config.get("interval_seconds", 30))
    state.pop("stopped_at", None)
    if state["interval_seconds"] < 30:
        raise ValueError("Screenshot interval must be at least 30 seconds")
    output = state_dir / "origins-latest.jpg"
    stop = state_dir / "stop.request"
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.GetForegroundWindow.restype = wintypes.HWND
    next_capture = time.monotonic()
    write_status(state_dir, state, status="watching")
    while not stop.exists():
        if time.monotonic() < next_capture:
            time.sleep(min(0.5, next_capture - time.monotonic()))
            continue
        started = time.monotonic()
        next_capture = started + state["interval_seconds"]
        ui = None
        try:
            if state.get("pending"):
                ui = DiscordWindow(args.port)
                pending = state["pending"]
                messages = confirmed_messages(ui, pending, config)
                if pending["phase"] == "sending" and messages:
                    write_status(state_dir, state, status="watching", pending=None,
                                 sent_count=state.get("sent_count", 0) + 1,
                                 last_sent_at=now(), last_message=messages[-1]["id"], error=None)
                else:
                    write_status(state_dir, state, status="paused_uncertain_upload",
                                 error="Unconfirmed attachment; inspect Discord before restarting")
                continue
            windows = game_windows()
            if not windows:
                write_status(state_dir, state, status="waiting_for_origins", error=None)
                continue
            for window in windows:
                if stop.exists():
                    break
                ui = DiscordWindow(args.port)
                navigate(ui, config)
                foreground = user32.GetForegroundWindow()
                result = subprocess.run(
                    [sys.executable, str(Path(__file__).resolve()), "--capture-hwnd", str(window["hwnd"]),
                     "--output", str(output)], timeout=18,
                    stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, creationflags=0x08000000,
                )
                if result.returncode:
                    detail = result.stderr.decode("utf-8", errors="replace").strip()
                    raise RuntimeError(detail[-800:] or "Window closed or did not produce a usable frame")
                write_status(state_dir, state, status="captured", last_capture_at=now(),
                             last_window=window, screenshot_bytes=output.stat().st_size,
                             foreground_before=int(foreground or 0))
                if stop.exists():
                    break
                send_capture(ui, config, output, state_dir, state)
                write_status(state_dir, state, foreground_after=int(user32.GetForegroundWindow() or 0))
                ui.close()
                ui = None
        except Exception as error:
            write_status(state_dir, state,
                         status="paused_uncertain_upload" if state.get("pending") else "retrying",
                         error=f"{type(error).__name__}: {error}"[:1200])
        finally:
            if ui is not None:
                ui.close()
        if time.monotonic() > next_capture:
            next_capture = time.monotonic() + state["interval_seconds"]
    write_status(state_dir, state, status="stopped", stopped_at=now())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=19222)
    parser.add_argument("--state-dir", type=Path)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--inspect", action="store_true")
    parser.add_argument("--evaluate")
    parser.add_argument("--capture-hwnd", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.capture_hwnd:
        capture_window(args.capture_hwnd, args.output)
        return
    if args.inspect or args.evaluate:
        ui = DiscordWindow(args.port)
        try:
            expression = args.evaluate or "({url:location.href,windows:document.title})"
            print(json.dumps(ui.evaluate(expression), ensure_ascii=True))
        finally:
            ui.close()
        return
    if not args.state_dir or not args.config:
        parser.error("--state-dir and --config are required for the sender")
    run_sender(args)


if __name__ == "__main__":
    main()
