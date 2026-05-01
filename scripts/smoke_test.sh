#!/usr/bin/env bash
# smoke_test.sh — M3+ 冒烟测试
# 验证 daemon → hook → audit → D001 语义 → interactive override → config
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$ROOT/target/debug"

cleanup() {
    kill "$DAEMON_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== building ==="
cargo build --manifest-path "$ROOT/Cargo.toml" 2>&1

echo "=== starting daemon (guard mode) ==="
HOME_OVERRIDE="/tmp/checkpoint-smoke-home-$$"
mkdir -p "$HOME_OVERRIDE/.checkpoint"
export HOME="$HOME_OVERRIDE"
export CHECKPOINT_ASSUME_NO_TTY=1
rm -f "$HOME_OVERRIDE/.checkpoint/ipc.sock" "$HOME_OVERRIDE/.checkpoint/audit.db" "$HOME_OVERRIDE/.checkpoint/state.json" "$HOME_OVERRIDE/.checkpoint/config.toml"

"$BIN/checkpointd" &
DAEMON_PID=$!
sleep 1

if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo "FAIL: daemon did not start"
    exit 1
fi
echo "daemon started (pid $DAEMON_PID)"

FAILED=0

echo ""
echo "=== test 1: Edit .env → expect deny (D006) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/.env","old_string":"OLD","new_string":"NEW","replace_all":false}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: got deny"
else
    echo "FAIL: expected deny, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 2: Write normal file → expect allow (no output) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"/tmp/test/hello.txt","content":"hello"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: no output (allow)"
else
    echo "FAIL: expected empty output, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 3: D001 git push origin main → expect ask ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: got ask for main push"
else
    echo "FAIL: expected ask, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 4: D001 git push origin feature/foo → expect allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin feature/login"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: feature branch push allowed"
else
    echo "FAIL: expected allow, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 5: D002 git push --force → expect deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push --force origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: force push denied"
else
    echo "FAIL: expected deny, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 5b: D002 git push -f origin main → expect deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push -f origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: short force flag denied"
else
    echo "FAIL: expected deny for -f, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 6: D003 rm -rf → expect deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/important"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: rm -rf denied"
else
    echo "FAIL: expected deny, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 6b: D003 rm -fr → expect deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -fr /tmp/important"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: rm -fr denied"
else
    echo "FAIL: expected deny for rm -fr, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 6c: quoted command text should not trigger D001 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo \"git push origin main\""}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: quoted text does not trigger git push matcher"
else
    echo "FAIL: expected allow for quoted text, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 6d: echo sudo should not trigger D004 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo sudo"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: echo sudo does not trigger D004"
else
    echo "FAIL: expected allow for echo sudo, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 6e: echo npm install should not trigger D010 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo npm install express"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: echo npm install does not trigger D010"
else
    echo "FAIL: expected allow for echo npm install, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 7: audit.db has events + decisions + rule_id ==="
EVENTS=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM events;")
DECISIONS=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM decisions;")

echo "events: $EVENTS, decisions: $DECISIONS"
if [ "$EVENTS" -ge 8 ] && [ "$DECISIONS" -ge 8 ]; then
    echo "PASS: audit records present"
else
    echo "FAIL: expected >= 8 events and >= 8 decisions"
    FAILED=1
fi

RULE_ID=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT rule_id FROM decisions WHERE rule_id IS NOT NULL LIMIT 1;")
if [ -n "$RULE_ID" ]; then
    echo "PASS: rule_id recorded ($RULE_ID)"
else
    echo "FAIL: no rule_id in decisions"
    FAILED=1
fi

echo ""
echo "=== test 8: status reads mode from state.json ==="
STATE_FILE="$HOME_OVERRIDE/.checkpoint/state.json"
if [ -f "$STATE_FILE" ]; then
    MODE_IN_STATE=$(python3 -c "import json; print(json.load(open('$STATE_FILE'))['mode'])" 2>/dev/null || echo "")
    if [ "$MODE_IN_STATE" = "guard" ]; then
        echo "PASS: state.json has mode=guard"
    else
        echo "FAIL: state.json mode=$MODE_IN_STATE, expected guard"
        FAILED=1
    fi
else
    echo "FAIL: state.json not found"
    FAILED=1
fi

echo ""
echo "=== test 9: config.toml auto-created by daemon ==="
CONFIG_FILE="$HOME_OVERRIDE/.checkpoint/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    if grep -q 'guard' "$CONFIG_FILE"; then
        echo "PASS: config.toml created with mode=guard"
    else
        echo "FAIL: config.toml exists but mode is not guard"
        cat "$CONFIG_FILE"
        FAILED=1
    fi
else
    echo "FAIL: config.toml not created by daemon"
    FAILED=1
fi

# ====== --interactive 测试：在 guard 模式下真正测 ask/deny 路径 ======
# 脚本没有 /dev/tty，read_yes_tty() 返回 false (N)。
# ask 分支：N → 输出 deny JSON + 发 override (ask → deny)
# deny 分支：N → 输出 deny JSON + 不发 override (deny 就是 deny)

echo ""
echo "=== test 10: --interactive ask (D001 main push) → no tty → deny output ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" --interactive 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: interactive ask → no tty → deny output"
else
    echo "FAIL: expected deny output from interactive ask, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 11: override decision written to audit (ask → deny, rule_id=user_override) ==="
OVERRIDE_COUNT=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM decisions WHERE rule_id = 'user_override';")
if [ "$OVERRIDE_COUNT" -ge 1 ]; then
    OVERRIDE_NOTE=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT note FROM decisions WHERE rule_id = 'user_override' LIMIT 1;")
    echo "PASS: override found in audit (note: $OVERRIDE_NOTE)"
else
    echo "FAIL: no override decision found in audit"
    FAILED=1
fi

echo ""
echo "=== test 12: --interactive deny (D006 .env) → no tty → deny output, no override ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/.env","old_string":"OLD","new_string":"NEW","replace_all":false}}' \
    | "$BIN/checkpoint-hook" --interactive 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: interactive deny → no tty → deny stands"
else
    echo "FAIL: expected deny from interactive deny, got: $RESP"
    FAILED=1
fi

# deny 保持不变不应产生新 override
OVERRIDE_AFTER_DENY=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM decisions WHERE rule_id = 'user_override';")
if [ "$OVERRIDE_AFTER_DENY" -eq "$OVERRIDE_COUNT" ]; then
    echo "PASS: no extra override for upheld deny"
else
    echo "FAIL: unexpected override count change ($OVERRIDE_COUNT → $OVERRIDE_AFTER_DENY)"
    FAILED=1
fi

echo ""
echo "=== test 13: non-interactive unchanged (allow path still silent) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"/tmp/test/normal.txt","content":"ok"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: non-interactive still silent on allow"
else
    echo "FAIL: non-interactive should be silent, got: $RESP"
    FAILED=1
fi

# ====== observer 模式 ======

echo ""
echo "=== test 14: observer mode via config.toml → allow + log ==="
kill "$DAEMON_PID" 2>/dev/null || true
sleep 0.5
rm -f "$HOME_OVERRIDE/.checkpoint/ipc.sock" "$HOME_OVERRIDE/.checkpoint/audit.db" "$HOME_OVERRIDE/.checkpoint/state.json"

echo 'mode = "observer"' > "$HOME_OVERRIDE/.checkpoint/config.toml"

"$BIN/checkpointd" &
DAEMON_PID=$!
sleep 1

RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/.env","old_string":"OLD","new_string":"NEW","replace_all":false}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: observer allows without output"
else
    echo "FAIL: observer should allow silently, got: $RESP"
    FAILED=1
fi

OBS_NOTE=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT note FROM decisions WHERE note LIKE '%observer%' LIMIT 1;")
if echo "$OBS_NOTE" | grep -q "observer"; then
    echo "PASS: observer note recorded in audit"
else
    echo "FAIL: expected observer note in audit, got: $OBS_NOTE"
    FAILED=1
fi

echo ""
echo "=== test 15: --interactive in observer mode → allow path ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/.env","old_string":"OLD","new_string":"NEW","replace_all":false}}' \
    | "$BIN/checkpoint-hook" --interactive 2>/dev/null)

if echo "$RESP" | grep -q "allowed"; then
    echo "PASS: interactive observer mode prints allowed"
else
    echo "FAIL: interactive observer should print allowed, got: $RESP"
    FAILED=1
fi

# ====== Autonomous 模式 ======

echo ""
echo "=== test 16: autonomous mode: D001 (main push) inactive → allow ==="
kill "$DAEMON_PID" 2>/dev/null || true
sleep 0.5
rm -f "$HOME_OVERRIDE/.checkpoint/ipc.sock" "$HOME_OVERRIDE/.checkpoint/audit.db" "$HOME_OVERRIDE/.checkpoint/state.json"

echo 'mode = "autonomous"' > "$HOME_OVERRIDE/.checkpoint/config.toml"

"$BIN/checkpointd" &
DAEMON_PID=$!
sleep 1

RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: autonomous allows main push (D001 inactive)"
else
    echo "FAIL: expected allow in autonomous for main push, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 17: autonomous mode: D002 (force push) active → deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push --force origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: autonomous denies force push (D002 active)"
else
    echo "FAIL: expected deny, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 18: autonomous mode: D004 (sudo) active → deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"sudo apt install something"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: autonomous denies sudo (D004 active)"
else
    echo "FAIL: expected deny for sudo, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 19: autonomous mode: D006 (sensitive file) active → deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/.env","old_string":"x","new_string":"y","replace_all":false}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: autonomous denies sensitive file write (D006 active)"
else
    echo "FAIL: expected deny, got: $RESP"
    FAILED=1
fi

# ====== Paranoid 模式 ======

echo ""
echo "=== test 20: paranoid mode: unknown Bash → ask (D999) ==="
kill "$DAEMON_PID" 2>/dev/null || true
sleep 0.5
rm -f "$HOME_OVERRIDE/.checkpoint/ipc.sock" "$HOME_OVERRIDE/.checkpoint/audit.db" "$HOME_OVERRIDE/.checkpoint/state.json"

echo 'mode = "paranoid"' > "$HOME_OVERRIDE/.checkpoint/config.toml"

"$BIN/checkpointd" &
DAEMON_PID=$!
sleep 1

RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hello"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: paranoid asks for unknown bash command (D999)"
else
    echo "FAIL: expected ask for unclassified operation, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 21: paranoid mode: unknown Write → ask (D999) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"/tmp/test/harmless.txt","content":"hello"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: paranoid asks for unknown write (D999)"
else
    echo "FAIL: expected ask, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 22: paranoid mode: D007 (secret in content) → deny ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"/tmp/test/config.json","old_string":"x","new_string":"api_key=TEST_SECRET_VALUE","replace_all":false}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: paranoid denies secret in content (D007)"
else
    echo "FAIL: expected deny for secret content, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 23: D010 (npm install) → ask in paranoid ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"npm install express"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: paranoid asks for npm install (D010)"
else
    echo "FAIL: expected ask for npm install, got: $RESP"
    FAILED=1
fi

# ====== CLI mode 子命令 ======

echo ""
echo "=== test 24: CLI mode subcommand ==="
MODE_OUT=$("$BIN/checkpoint" mode)
if [ "$MODE_OUT" = "paranoid" ]; then
    echo "PASS: checkpoint mode reads paranoid from config.toml"
else
    echo "FAIL: expected paranoid, got: $MODE_OUT"
    FAILED=1
fi

"$BIN/checkpoint" mode autonomous > /dev/null
MODE_OUT=$("$BIN/checkpoint" mode)
if [ "$MODE_OUT" = "autonomous" ]; then
    echo "PASS: checkpoint mode autonomous written and read back"
else
    echo "FAIL: expected autonomous after set, got: $MODE_OUT"
    FAILED=1
fi

"$BIN/checkpoint" mode guard > /dev/null
MODE_OUT=$("$BIN/checkpoint" mode)
if [ "$MODE_OUT" = "guard" ]; then
    echo "PASS: checkpoint mode guard written and read back"
else
    echo "FAIL: expected guard after set, got: $MODE_OUT"
    FAILED=1
fi

if grep -q 'mode = "guard"' "$HOME_OVERRIDE/.checkpoint/config.toml"; then
    echo "PASS: config.toml contains mode = \"guard\""
else
    echo "FAIL: config.toml does not have expected content"
    cat "$HOME_OVERRIDE/.checkpoint/config.toml"
    FAILED=1
fi

echo ""
echo "=== test 27: daemon hot-reload guard (from paranoid) → D001 ask ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: hot-reload to guard, D001 ask"
else
    echo "FAIL: expected ask after hot-reload to guard, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 28: daemon hot-reload observer → D001 allow ==="
"$BIN/checkpoint" mode observer > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: hot-reload to observer, D001 allowed"
else
    echo "FAIL: expected allow after hot-reload to observer, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 29: daemon hot-reload autonomous → D001 allow ==="
"$BIN/checkpoint" mode autonomous > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: hot-reload to autonomous, D001 inactive → allow"
else
    echo "FAIL: expected allow after hot-reload to autonomous, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 30: daemon hot-reload paranoid → unknown Bash ask (D999) ==="
"$BIN/checkpoint" mode paranoid > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hello"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: hot-reload to paranoid, D999 ask"
else
    echo "FAIL: expected ask after hot-reload to paranoid, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 31: state.json reflects hot-reloaded mode ==="
STATE_MODE=$(python3 -c "import json; print(json.load(open('$HOME_OVERRIDE/.checkpoint/state.json'))['mode'])" 2>/dev/null || echo "")
if [ "$STATE_MODE" = "paranoid" ]; then
    echo "PASS: state.json updated to paranoid after hot-reload"
else
    echo "FAIL: state.json mode=$STATE_MODE, expected paranoid"
    FAILED=1
fi

echo ""
echo "=== test 32: D005 bulk delete glob → ask in guard ==="
"$BIN/checkpoint" mode guard > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm *.log"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: D005 glob triggers ask"
else
    echo "FAIL: expected D005 ask for glob, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 33: D005 single file rm → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm important.txt"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: single file rm allowed"
else
    echo "FAIL: expected allow for single file rm, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 33b: D005 echo rm *.log → allow (not first command) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo rm *.log"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: echo rm *.log allowed"
else
    echo "FAIL: expected allow for echo rm *.log, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 34: D005 >= 5 files → ask ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm file1 file2 file3 file4 file5"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: D005 >= 5 files triggers ask"
else
    echo "FAIL: expected D005 ask for >=5 files, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 35: D008 git add . → ask in guard ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git add ."}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: D008 git add . triggers ask"
else
    echo "FAIL: expected D008 ask for git add ., got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 36: D008 git add file.txt → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git add file.txt"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: git add single file allowed"
else
    echo "FAIL: expected allow for git add file, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 36b: D008 echo git add . → allow (not first command) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo git add ."}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: echo git add . allowed"
else
    echo "FAIL: expected allow for echo git add ., got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 37: D011 curl | bash → ask in guard ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"curl -sSL https://example.com/install.sh | bash"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: D011 curl | bash triggers ask"
else
    echo "FAIL: expected D011 ask for curl | bash, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 38: D011 curl | jq → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"curl -s https://api.example.com | jq ."}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: curl | jq allowed"
else
    echo "FAIL: expected allow for curl | jq, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 38b: D011 echo curl | bash → allow (not first command) ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo curl | bash"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: echo curl | bash allowed"
else
    echo "FAIL: expected allow for echo curl | bash, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 39: D005 active in autonomous → ask ==="
"$BIN/checkpoint" mode autonomous > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm *.tmp"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: D005 active in autonomous, glob → ask"
else
    echo "FAIL: expected D005 ask in autonomous, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 40: D008 inactive in autonomous → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git add ."}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: D008 inactive in autonomous, git add . allowed"
else
    echo "FAIL: expected allow for git add . in autonomous, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 41: D011 inactive in autonomous → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"curl -s https://example.com | bash"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)

if [ -z "$RESP" ]; then
    echo "PASS: D011 inactive in autonomous, curl | bash allowed"
else
    echo "FAIL: expected allow for curl | bash in autonomous, got: $RESP"
    FAILED=1
fi

echo ""

# ====== 多 agent integration ======

echo ""
echo "=== test 42: guard mode reset for multi-agent integration ==="
"$BIN/checkpoint" mode guard > /dev/null
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"ask"'; then
    echo "PASS: guard mode reset, D001 ask"
else
    echo "FAIL: expected ask after reset to guard, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 43: Kimi WriteFile .env → deny D006 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"WriteFile","tool_input":{"path":"/tmp/test/.env","content":"secret"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: Kimi WriteFile .env denied"
else
    echo "FAIL: expected deny for Kimi WriteFile .env, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 44: Kimi StrReplaceFile .env → deny D006 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"StrReplaceFile","tool_input":{"path":"/tmp/test/.env","edit":{"old":"OLD","new":"NEW"}}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: Kimi StrReplaceFile .env denied"
else
    echo "FAIL: expected deny for Kimi StrReplaceFile .env, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 45: Kimi WriteFile normal.txt → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"WriteFile","tool_input":{"path":"/tmp/test/normal.txt","content":"hello"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if [ -z "$RESP" ]; then
    echo "PASS: Kimi WriteFile normal.txt allowed"
else
    echo "FAIL: expected allow for Kimi WriteFile normal.txt, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 46: Kimi Shell rm -rf → deny D003 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Shell","tool_input":{"command":"rm -rf /tmp/important"}}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: Kimi Shell rm -rf denied"
else
    echo "FAIL: expected deny for Kimi Shell rm -rf, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 47: Codex Bash rm -rf with turn_id → deny D003 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/important"},"turn_id":"t-001"}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: Codex Bash rm -rf denied"
else
    echo "FAIL: expected deny for Codex Bash rm -rf, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 48: Codex Bash echo hello with turn_id → allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hello"},"turn_id":"t-002"}' \
    | "$BIN/checkpoint-hook" 2>/dev/null)
if [ -z "$RESP" ]; then
    echo "PASS: Codex Bash echo hello allowed"
else
    echo "FAIL: expected allow for Codex Bash echo hello, got: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 49: audit.db agent field has kimi_code and codex_cli ==="
KIMI_COUNT=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM events WHERE agent = 'kimi_code';")
CODEX_COUNT=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT COUNT(*) FROM events WHERE agent = 'codex_cli';")
echo "kimi_code events: $KIMI_COUNT, codex_cli events: $CODEX_COUNT"
if [ "$KIMI_COUNT" -ge 1 ] && [ "$CODEX_COUNT" -ge 1 ]; then
    echo "PASS: audit.db has both kimi_code and codex_cli"
else
    echo "FAIL: expected >=1 kimi_code and >=1 codex_cli events"
    FAILED=1
fi

echo ""
echo "=== test 50: CHECKPOINT_AGENT=kimi + Claude Bash payload → env override to kimi_code, deny D003 ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/important"}}' \
    | CHECKPOINT_AGENT=kimi "$BIN/checkpoint-hook" 2>/dev/null)
if echo "$RESP" | grep -q '"deny"'; then
    echo "PASS: CHECKPOINT_AGENT=kimi env override accepted, deny D003"
else
    echo "FAIL: expected deny with CHECKPOINT_AGENT=kimi, got: $RESP"
    FAILED=1
fi
LAST_AGENT=$(sqlite3 "$HOME_OVERRIDE/.checkpoint/audit.db" "SELECT agent FROM events ORDER BY rowid DESC LIMIT 1;")
if [ "$LAST_AGENT" = "kimi_code" ]; then
    echo "PASS: audit.db last event agent=kimi_code"
else
    echo "FAIL: expected last event agent=kimi_code, got: $LAST_AGENT"
    FAILED=1
fi

echo ""
echo "=== test 51: CHECKPOINT_AGENT=gemini + Claude Bash echo hello → stderr warning, allow ==="
RESP=$(echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hello"}}' \
    | CHECKPOINT_AGENT=gemini "$BIN/checkpoint-hook" 2>/tmp/smoke_test_gemini_stderr.txt)
if [ -z "$RESP" ]; then
    echo "PASS: CHECKPOINT_AGENT=gemini allows after fallback"
else
    echo "FAIL: expected allow for CHECKPOINT_AGENT=gemini, got: $RESP"
    FAILED=1
fi
if grep -q "not yet supported" /tmp/smoke_test_gemini_stderr.txt 2>/dev/null; then
    echo "PASS: stderr contains 'not yet supported' warning"
else
    echo "FAIL: expected stderr warning for unsupported agent"
    cat /tmp/smoke_test_gemini_stderr.txt
    FAILED=1
fi
rm -f /tmp/smoke_test_gemini_stderr.txt

echo ""
if [ "$FAILED" -eq 0 ]; then
    echo "=== ALL TESTS PASSED ==="
else
    echo "=== SOME TESTS FAILED ==="
    exit 1
fi
