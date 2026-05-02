#!/usr/bin/env bash
# provider_verify.sh — Provider Runner 真机验证
# 测试 Claude/Kimi/Codex agent_prompt new + resume
set -euo pipefail

TOKEN=$(cat ~/.agent-aspect/bridge.token 2>/dev/null || true)
if [ -z "$TOKEN" ]; then
    echo "FAIL: no bridge token found"
    exit 1
fi

API="http://127.0.0.1:7676"
AUTH="Authorization: Bearer $TOKEN"

# Check bridge is running
if ! curl -s "$API/health" | grep -q '"status":"ok"'; then
    echo "FAIL: bridge not running"
    exit 1
fi

# Find a known project_path from conversations
PROJECT=$(curl -s -H "$AUTH" "$API/run/context" | python3 -c "
import json,sys
d=json.load(sys.stdin)
projects=d.get('projects',[])
if projects:
    print(projects[0]['path'])
else:
    print('')
" 2>/dev/null)

if [ -z "$PROJECT" ]; then
    echo "WARN: no known project_path, using HOME"
    PROJECT="$HOME"
fi

echo "=== Provider Runner Verification ==="
echo "Bridge: $API"
echo "Project: $PROJECT"
echo ""

FAILED=0

# --- Claude ---
echo "=== test 1: Claude agent_prompt (new session) ==="
RESP=$(curl -s -X POST \
    -H "$AUTH" -H "Content-Type: application/json" \
    -d "{\"kind\":\"agent_prompt\",\"provider\":\"claude_code\",\"project_path\":\"$PROJECT\",\"prompt\":\"echo hello from agent-aspect test\"}" \
    "$API/jobs")
JOB_ID=$(echo "$RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('job_id',''))" 2>/dev/null)
if [ -n "$JOB_ID" ]; then
    echo "Queued job: $JOB_ID"
    # Wait for completion
    for i in $(seq 1 30); do
        STATUS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)
        if [ "$STATUS" = "succeeded" ] || [ "$STATUS" = "failed" ] || [ "$STATUS" = "cancelled" ]; then
            break
        fi
        sleep 2
    done
    echo "Final status: $STATUS"
    if [ "$STATUS" = "succeeded" ]; then
        echo "PASS: Claude agent_prompt succeeded"
    else
        LOGS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID/logs" 2>/dev/null | python3 -c "
import json,sys
d=json.load(sys.stdin)
for l in d.get('logs',[])[:5]:
    print(l.get('chunk','')[:200])
" 2>/dev/null)
        echo "FAIL: Claude agent_prompt $STATUS"
        echo "Logs: $LOGS"
        FAILED=1
    fi
else
    echo "FAIL: no job_id in response: $RESP"
    FAILED=1
fi

echo ""
echo "=== test 2: Claude agent_prompt (resume) ==="
# Get a conversation_id from the project
CONV_ID=$(curl -s -H "$AUTH" "$API/run/context" | python3 -c "
import json,sys
d=json.load(sys.stdin)
for c in d.get('recent_conversations',[]):
    if c.get('conversation_id'):
        print(c['conversation_id'])
        break
" 2>/dev/null || echo "")

if [ -n "$CONV_ID" ]; then
    RESP=$(curl -s -X POST \
        -H "$AUTH" -H "Content-Type: application/json" \
        -d "{\"kind\":\"agent_prompt\",\"provider\":\"claude_code\",\"project_path\":\"$PROJECT\",\"conversation_id\":\"$CONV_ID\",\"prompt\":\"list files\"}" \
        "$API/jobs")
    JOB_ID=$(echo "$RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('job_id',''))" 2>/dev/null)
    if [ -n "$JOB_ID" ]; then
        echo "Queued resume job: $JOB_ID (conv: $CONV_ID)"
        for i in $(seq 1 30); do
            STATUS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)
            if [ "$STATUS" = "succeeded" ] || [ "$STATUS" = "failed" ] || [ "$STATUS" = "cancelled" ]; then
                break
            fi
            sleep 2
        done
        echo "Resume status: $STATUS"
        if [ "$STATUS" = "succeeded" ]; then
            echo "PASS: Claude resume succeeded"
        else
            echo "INFO: Claude resume $STATUS (may fail if session expired)"
        fi
    else
        echo "INFO: no job_id for resume: $RESP"
    fi
else
    echo "SKIP: no conversation_id available for resume test"
fi

# --- Kimi ---
echo ""
echo "=== test 3: Kimi agent_prompt (new session) ==="
RESP=$(curl -s -X POST \
    -H "$AUTH" -H "Content-Type: application/json" \
    -d "{\"kind\":\"agent_prompt\",\"provider\":\"kimi_code\",\"project_path\":\"$PROJECT\",\"prompt\":\"echo hello from agent-aspect\"}" \
    "$API/jobs")
JOB_ID=$(echo "$RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('job_id',''))" 2>/dev/null)
if [ -n "$JOB_ID" ]; then
    echo "Queued job: $JOB_ID"
    for i in $(seq 1 30); do
        STATUS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)
        if [ "$STATUS" = "succeeded" ] || [ "$STATUS" = "failed" ] || [ "$STATUS" = "cancelled" ]; then
            break
        fi
        sleep 2
    done
    echo "Final status: $STATUS"
    if [ "$STATUS" = "succeeded" ]; then
        echo "PASS: Kimi agent_prompt succeeded"
    else
        LOGS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID/logs" 2>/dev/null | python3 -c "
import json,sys
d=json.load(sys.stdin)
for l in d.get('logs',[])[:5]:
    print(l.get('chunk','')[:200])
" 2>/dev/null)
        echo "INFO: Kimi agent_prompt $STATUS"
        echo "Logs: $LOGS"
    fi
else
    echo "FAIL: no job_id: $RESP"
    FAILED=1
fi

# --- Codex (best-effort) ---
echo ""
echo "=== test 4: Codex agent_prompt (best-effort) ==="
RESP=$(curl -s -X POST \
    -H "$AUTH" -H "Content-Type: application/json" \
    -d "{\"kind\":\"agent_prompt\",\"provider\":\"codex_cli\",\"project_path\":\"$PROJECT\",\"prompt\":\"echo hello\"}" \
    "$API/jobs")
JOB_ID=$(echo "$RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('job_id',''))" 2>/dev/null)
if [ -n "$JOB_ID" ]; then
    echo "Queued job: $JOB_ID"
    for i in $(seq 1 30); do
        STATUS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)
        if [ "$STATUS" = "succeeded" ] || [ "$STATUS" = "failed" ] || [ "$STATUS" = "cancelled" ]; then
            break
        fi
        sleep 2
    done
    echo "Final status: $STATUS"
    LOGS=$(curl -s -H "$AUTH" "$API/jobs/$JOB_ID/logs" 2>/dev/null | python3 -c "
import json,sys
d=json.load(sys.stdin)
for l in d.get('logs',[])[:5]:
    print(l.get('chunk','')[:200])
" 2>/dev/null)
    if [ "$STATUS" = "succeeded" ]; then
        echo "PASS: Codex agent_prompt succeeded"
    else
        echo "INFO: Codex agent_prompt $STATUS (best-effort, failure is acceptable)"
        echo "Logs: $LOGS"
    fi
else
    echo "FAIL: no job_id: $RESP"
    FAILED=1
fi

echo ""
echo "=== Provider Verification Complete ==="
if [ "$FAILED" -eq 0 ]; then
    echo "All critical tests passed"
else
    echo "Some critical tests failed"
fi
