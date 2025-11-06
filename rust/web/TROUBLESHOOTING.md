# Troubleshooting Guide

Common issues and solutions for axm_web server.

## Table of Contents

- [Server Issues](#server-issues)
- [Connection Problems](#connection-problems)
- [Session Issues](#session-issues)
- [Performance Issues](#performance-issues)
- [Static File Issues](#static-file-issues)
- [Error Messages](#error-messages)
- [Debugging Techniques](#debugging-techniques)
- [Getting Help](#getting-help)

---

## Server Issues

### Server Won't Start

**Problem:** Server fails to start with error message.

**Common Causes:**

1. **Port Already in Use**

```
Error: Failed to bind to address: Address already in use (os error 48)
```

**Solution:**
```bash
# Check what's using the port
netstat -tuln | grep 8080    # Linux/Mac
netstat -ano | findstr 8080  # Windows

# Kill the process
kill -9 <PID>                # Linux/Mac
taskkill /F /PID <PID>       # Windows

# Or use a different port
cargo run -p axm_cli -- serve --port 3000
```

2. **Static Directory Not Found**

```
Error: Configuration error: static directory does not exist
```

**Solution:**
```bash
# Create static directory
mkdir -p static

# Or specify existing directory
cargo run -p axm_cli -- serve --static-dir /path/to/static
```

3. **Permission Denied**

```
Error: Failed to bind to address: Permission denied (os error 13)
```

**Solution:**
```bash
# Use port > 1024 (no root required)
cargo run -p axm_cli -- serve --port 8080

# Or use root (not recommended)
sudo cargo run -p axm_cli -- serve --port 80
```

### Server Crashes Immediately

**Problem:** Server starts but crashes within seconds.

**Diagnostic Steps:**

```bash
# Run with debug logging
RUST_LOG=debug cargo run -p axm_cli -- serve

# Check for panic messages
RUST_BACKTRACE=1 cargo run -p axm_cli -- serve

# Verify dependencies
cargo check -p axm_web
```

**Common Causes:**
- Missing dependencies (run `cargo build`)
- Corrupted build (run `cargo clean && cargo build`)
- Invalid configuration file

### Server Becomes Unresponsive

**Problem:** Server stops responding to requests.

**Diagnostic Steps:**

```bash
# Check if process is running
ps aux | grep axm

# Check CPU/memory usage
top | grep axm

# Check logs for errors
tail -f /var/log/axiomind/error.log
```

**Solutions:**

1. **High memory usage:** Increase memory limit or restart
```bash
# Restart systemd service
sudo systemctl restart axm-web
```

2. **Deadlock/hanging:** Enable debug logging and check for stuck operations
```bash
RUST_LOG=trace cargo run -p axm_cli -- serve 2>&1 | tee debug.log
```

---

## Connection Problems

### Cannot Connect to Server

**Problem:** Browser cannot connect to `http://localhost:8080`

**Diagnostic Steps:**

```bash
# Verify server is running
curl http://localhost:8080/api/health

# Check port binding
netstat -tuln | grep 8080

# Test with different client
curl -v http://localhost:8080
```

**Solutions:**

1. **Server not running:** Start the server
```bash
cargo run -p axm_cli -- serve
```

2. **Wrong port:** Check which port server is using
```bash
# Look for "Server running on" message in logs
# Or check with netstat
```

3. **Firewall blocking:** Allow port through firewall
```bash
# Linux (ufw)
sudo ufw allow 8080

# Windows
# Add inbound rule in Windows Firewall
```

### SSE Connection Fails

**Problem:** Real-time events not streaming, "EventSource failed" error.

**Browser Console Error:**
```
EventSource connection failed: net::ERR_INCOMPLETE_CHUNKED_ENCODING
```

**Diagnostic Steps:**

```bash
# Test SSE endpoint directly
curl -N http://localhost:8080/api/sessions/{session_id}/events

# Check server logs for errors
RUST_LOG=axm_web::handlers::sse=debug cargo run -p axm_cli -- serve
```

**Solutions:**

1. **Session doesn't exist:** Create session first
```bash
# Create session and note session_id
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1}'
```

2. **Reverse proxy buffering:** Disable buffering in nginx
```nginx
location /api/sessions/ {
    proxy_buffering off;
    proxy_read_timeout 3600s;
}
```

3. **Browser limit:** Close other SSE connections (browsers limit to ~6 per domain)

### Intermittent Connection Drops

**Problem:** Connection drops and reconnects frequently.

**Diagnostic Steps:**

```bash
# Monitor network with netstat
watch -n 1 'netstat -an | grep 8080'

# Check system logs for network issues
dmesg | grep -i network
```

**Solutions:**

1. **Network instability:** Check network quality
2. **Server timeout:** Increase timeout settings
3. **Resource limits:** Increase file descriptor limits
```bash
ulimit -n 4096
```

---

## Session Issues

### Session Not Found Error

**Problem:** API returns `404 Session not found`

**Error Response:**
```json
{
  "error": "Session not found",
  "code": "SESSION_NOT_FOUND",
  "details": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**Solutions:**

1. **Session expired:** Create new session
```bash
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1}'
```

2. **Server restarted:** Sessions are in-memory, restart clears all sessions
3. **Wrong session ID:** Verify session ID is correct

### Invalid Action Error

**Problem:** Action submission fails with `409 Invalid action`

**Error Response:**
```json
{
  "error": "Invalid action: Cannot check when facing a bet",
  "code": "INVALID_ACTION",
  "details": {
    "action": "Check",
    "current_bet": 100
  }
}
```

**Solutions:**

1. **Check game state first:**
```bash
curl http://localhost:8080/api/sessions/{session_id}/state
```

2. **Review available actions:**
Look at `available_actions` field in state response

3. **Not your turn:** Wait for `current_player` to match your player ID

### Session Won't Create

**Problem:** POST to `/api/sessions` fails

**Diagnostic Steps:**

```bash
# Check request format
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1}' \
  -v

# Check server logs
RUST_LOG=axm_web::session=debug cargo run -p axm_cli -- serve
```

**Common Causes:**

1. **Invalid JSON:** Ensure request body is valid JSON
2. **Invalid parameters:** level must be 1-10
3. **Engine initialization failed:** Check engine logs

---

## Performance Issues

### Slow Response Times

**Problem:** API requests take several seconds to complete.

**Diagnostic Steps:**

```bash
# Measure request time
time curl http://localhost:8080/api/sessions/{session_id}/state

# Check server load
top

# Enable performance logging
RUST_LOG=info cargo run -p axm_cli -- serve
```

**Solutions:**

1. **High CPU usage:** Reduce concurrent sessions or upgrade hardware
2. **Memory pressure:** Restart server to clear memory
3. **Slow AI opponent:** AI computation time varies (expected behavior)

### High Memory Usage

**Problem:** Server consumes excessive memory.

**Diagnostic Steps:**

```bash
# Check memory usage
ps aux | grep axm

# Monitor over time
watch -n 5 'ps aux | grep axm'
```

**Solutions:**

1. **Too many sessions:** Delete old sessions
```bash
curl -X DELETE http://localhost:8080/api/sessions/{session_id}
```

2. **Memory leak (suspected):** Restart server and report issue
3. **Set memory limit (systemd):**
```ini
[Service]
MemoryMax=500M
```

### SSE Events Delayed

**Problem:** Game events arrive seconds after actions.

**Diagnostic Steps:**

```bash
# Test event latency
curl -N http://localhost:8080/api/sessions/{session_id}/events

# Check server logs for processing time
RUST_LOG=axm_web::events=debug cargo run -p axm_cli -- serve
```

**Solutions:**

1. **Server overloaded:** Reduce concurrent sessions
2. **Network buffering:** Disable proxy buffering
3. **Browser throttling:** Check browser console for errors

---

## Static File Issues

### 404 for Static Files

**Problem:** `GET /styles.css` returns 404 Not Found

**Diagnostic Steps:**

```bash
# Check static directory
ls -la static/

# Verify server configuration
curl http://localhost:8080/api/health
```

**Solutions:**

1. **Files missing:** Ensure static files exist
```bash
# Copy static files
cp -r rust/web/static/* ./static/
```

2. **Wrong directory:** Specify correct directory
```bash
cargo run -p axm_cli -- serve --static-dir ./static
```

3. **Permissions:** Ensure files are readable
```bash
chmod -R 644 static/*
```

### Styles Not Loading

**Problem:** HTML loads but CSS/JS not applied

**Browser Console Error:**
```
Failed to load resource: net::ERR_FILE_NOT_FOUND
```

**Solutions:**

1. **Check file paths:** Ensure paths in HTML match actual file locations
2. **Check MIME types:** Server should serve CSS as `text/css`
3. **Clear browser cache:** Hard refresh (Ctrl+Shift+R)

### Card Images Not Displaying

**Problem:** Card images show broken image icon

**Diagnostic Steps:**

```bash
# Check if image files exist
ls static/assets/cards/

# Test image URL directly
curl http://localhost:8080/assets/cards/AS.svg
```

**Solutions:**

1. **Files missing:** Add card image assets
2. **Wrong path:** Check image src in HTML
3. **MIME type:** Ensure SVG served as `image/svg+xml`

---

## Error Messages

### "Engine Error" Messages

**Error:**
```json
{
  "error": "Game engine error: Invalid game state",
  "code": "ENGINE_ERROR"
}
```

**Solutions:**

1. **Report to developers:** This indicates a bug in game engine
2. **Workaround:** Create new session
3. **Check logs:** Look for detailed error in server logs

### "Session Expired" Messages

**Error:**
```json
{
  "error": "Session expired",
  "code": "SESSION_EXPIRED"
}
```

**Solutions:**

1. **Increase timeout:** Configure longer session timeout
2. **Create new session:** Old session is no longer valid
3. **Keep alive:** Send periodic requests to prevent expiration

### "Internal Server Error"

**Error:**
```
500 Internal Server Error
```

**Diagnostic Steps:**

```bash
# Check server logs immediately
tail -f /var/log/axiomind/error.log

# Enable debug logging
RUST_LOG=debug cargo run -p axm_cli -- serve

# Check for panics
RUST_BACKTRACE=full cargo run -p axm_cli -- serve
```

**Solutions:**

1. **Report bug:** Note exact steps to reproduce
2. **Restart server:** May resolve transient issues
3. **Check for updates:** Ensure running latest version

---

## Debugging Techniques

### Enable Debug Logging

**Full debug output:**
```bash
RUST_LOG=debug cargo run -p axm_cli -- serve
```

**Module-specific debug:**
```bash
# Session management only
RUST_LOG=axm_web::session=debug cargo run -p axm_cli -- serve

# Multiple modules
RUST_LOG=axm_web::session=debug,axm_web::events=trace cargo run -p axm_cli -- serve
```

**JSON formatted logs:**
```bash
RUST_LOG_FORMAT=json cargo run -p axm_cli -- serve
```

### Capture Request/Response Data

**Using curl verbose mode:**
```bash
curl -v http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1}'
```

**Browser DevTools:**
1. Open DevTools (F12)
2. Network tab
3. Filter by XHR/Fetch
4. Inspect request/response details

### Monitor SSE Events

**Terminal monitoring:**
```bash
curl -N http://localhost:8080/api/sessions/{session_id}/events | jq .
```

**Browser EventSource debugging:**
```javascript
const eventSource = new EventSource('/api/sessions/' + sessionId + '/events');

eventSource.onopen = () => console.log('SSE connected');
eventSource.onerror = (e) => console.error('SSE error:', e);
eventSource.addEventListener('game_event', (e) => {
  console.log('Event:', JSON.parse(e.data));
});
```

### Analyze Performance

**Request timing:**
```bash
# Time single request
time curl http://localhost:8080/api/sessions/{session_id}/state

# Load testing with ab
ab -n 100 -c 10 http://localhost:8080/api/health
```

**Memory profiling:**
```bash
# Install valgrind (Linux)
sudo apt install valgrind

# Run with valgrind
valgrind --leak-check=full ./target/release/axm serve
```

---

## Getting Help

### Gathering Diagnostic Information

When reporting issues, include:

1. **Server version:**
```bash
cargo pkgid axm_web
```

2. **Error messages:**
```bash
# Last 100 lines of logs
tail -n 100 /var/log/axiomind/error.log
```

3. **System information:**
```bash
# OS and version
uname -a

# Rust version
rustc --version

# Available memory
free -h
```

4. **Steps to reproduce:**
- Exact curl commands or UI actions
- Expected vs actual behavior
- Frequency (always, intermittent, rare)

### Check Existing Issues

Search for similar issues in:
- Project issue tracker
- Documentation
- Troubleshooting guide (this file)

### Report a Bug

Include in bug report:

- **Title:** Brief description of the issue
- **Environment:** OS, Rust version, server version
- **Steps to reproduce:** Detailed steps
- **Expected behavior:** What should happen
- **Actual behavior:** What actually happens
- **Logs:** Relevant log excerpts
- **Screenshots:** If UI-related

### Ask for Help

If troubleshooting doesn't resolve the issue:

1. Check project README for contact information
2. Open an issue on the project repository
3. Include diagnostic information (see above)
4. Be patient and provide requested information promptly

---

## Quick Reference

### Common Commands

```bash
# Start server with debug logging
RUST_LOG=debug cargo run -p axm_cli -- serve

# Test server health
curl http://localhost:8080/api/health

# Create session
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 1}'

# Get session state
curl http://localhost:8080/api/sessions/{session_id}/state

# Stream events
curl -N http://localhost:8080/api/sessions/{session_id}/events

# Delete session
curl -X DELETE http://localhost:8080/api/sessions/{session_id}
```

### Useful Log Filters

```bash
# Session-related logs
RUST_LOG=axm_web::session=debug

# Event streaming logs
RUST_LOG=axm_web::events=trace

# HTTP request logs
RUST_LOG=axm_web::middleware=debug

# All axm_web logs
RUST_LOG=axm_web=debug

# All logs including dependencies
RUST_LOG=trace
```

### Health Check

Quick server health check:
```bash
curl http://localhost:8080/api/health | jq .
```

Expected response:
```json
{
  "status": "healthy",
  "uptime_seconds": 3625,
  "active_sessions": 3,
  "total_hands_played": 142
}
```

---

## Additional Resources

- [README.md](README.md) - Setup and usage guide
- [API.md](API.md) - Complete API documentation
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
- [../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) - System architecture
- [../../docs/RUNBOOK.md](../../docs/RUNBOOK.md) - Operations runbook
