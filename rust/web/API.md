# API Documentation

Complete API reference for the axm_web HTTP server.

## Table of Contents

- [Overview](#overview)
- [Base URL](#base-url)
- [Authentication](#authentication)
- [Content Types](#content-types)
- [Error Responses](#error-responses)
- [Game Session Endpoints](#game-session-endpoints)
- [Server-Sent Events](#server-sent-events)
- [Hand History Endpoints](#hand-history-endpoints)
- [Settings Endpoints](#settings-endpoints)
- [Health Check](#health-check)
- [Static Assets](#static-assets)

## Overview

The axm_web API provides RESTful endpoints for:

- Creating and managing poker game sessions
- Submitting player actions (fold, call, raise, etc.)
- Streaming real-time game events via Server-Sent Events (SSE)
- Accessing hand history and statistics
- Configuring game settings

## Base URL

When running locally:
```
http://localhost:8080
```

Configure host and port via CLI:
```bash
cargo run -p axm_cli -- serve --host 127.0.0.1 --port 3000
```

## Authentication

Currently, no authentication is required. The server is designed for local single-user access.

**Future considerations**: Session tokens, API keys for multi-user deployments.

## Content Types

All endpoints accept and return JSON unless otherwise specified:

```
Content-Type: application/json
Accept: application/json
```

SSE endpoints return `text/event-stream`:
```
Content-Type: text/event-stream
```

## Error Responses

All errors follow a consistent format:

```json
{
  "error": "Session not found",
  "code": "SESSION_NOT_FOUND",
  "details": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### Common HTTP Status Codes

- `200 OK`: Successful GET request
- `201 Created`: Successful resource creation
- `202 Accepted`: Action accepted and processed
- `204 No Content`: Successful DELETE request
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Resource not found
- `409 Conflict`: Invalid game state for requested action
- `500 Internal Server Error`: Server-side error

### Error Codes

| Code | Description |
|------|-------------|
| `SESSION_NOT_FOUND` | Session ID does not exist |
| `INVALID_ACTION` | Action not allowed in current game state |
| `ENGINE_ERROR` | Game engine error |
| `SESSION_EXPIRED` | Session has timed out |
| `INVALID_REQUEST` | Malformed request body |
| `STATIC_FILE_NOT_FOUND` | Requested static asset not found |

---

## Game Session Endpoints

### Create Session

Create a new poker game session.

**Endpoint:** `POST /api/sessions`

**Request Body:**
```json
{
  "seed": 12345,              // Optional: RNG seed for reproducibility
  "level": 1,                 // Optional: Blind level (1-20), default: 1
  "opponent_type": {          // Optional: Opponent type
    "AI": "Baseline"          // AI with strategy name, or "Human"
  }
}
```

**Note:** Levels 21+ are treated as level 20.

**Response:** `201 Created`
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "config": {
    "seed": 12345,
    "level": 1,
    "opponent_type": {"AI": "Baseline"}
  },
  "state": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "players": [
      {
        "id": 0,
        "seat": "Button",
        "stack": 5000,
        "hole_cards": [
          {"rank": "Ace", "suit": "Spades"},
          {"rank": "King", "suit": "Hearts"}
        ],
        "is_active": true,
        "last_action": null,
        "bet": 0,
        "total_bet": 0,
        "is_all_in": false,
        "folded": false
      },
      {
        "id": 1,
        "seat": "BigBlind",
        "stack": 4900,
        "hole_cards": null,  // Hidden for opponent
        "is_active": false,
        "last_action": {"Bet": 100},
        "bet": 100,
        "total_bet": 100,
        "is_all_in": false,
        "folded": false
      }
    ],
    "board": [],
    "pot": 150,
    "current_player": 0,
    "available_actions": [
      {
        "action_type": "fold",
        "min_amount": null,
        "max_amount": null
      },
      {
        "action_type": "call",
        "min_amount": 100,
        "max_amount": 100
      },
      {
        "action_type": "raise",
        "min_amount": 200,
        "max_amount": 5000
      }
    ],
    "hand_id": "hand_001",
    "street": "Preflop"
  }
}
```

**Curl Example:**
```bash
curl -X POST http://localhost:8080/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"level": 2, "opponent_type": {"AI": "Baseline"}}'
```

---

### Get Session

Retrieve current session information.

**Endpoint:** `GET /api/sessions/{session_id}`

**Response:** `200 OK`
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "config": { ... },
  "state": { ... }
}
```

**Curl Example:**
```bash
curl http://localhost:8080/api/sessions/550e8400-e29b-41d4-a716-446655440000
```

---

### Get Session State

Retrieve only the current game state (lighter than full session).

**Endpoint:** `GET /api/sessions/{session_id}/state`

**Response:** `200 OK`
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "players": [ ... ],
  "board": [],
  "pot": 150,
  "current_player": 0,
  "available_actions": [ ... ],
  "hand_id": "hand_001",
  "street": "Preflop"
}
```

**Curl Example:**
```bash
curl http://localhost:8080/api/sessions/550e8400-e29b-41d4-a716-446655440000/state
```

---

### Submit Player Action

Submit a poker action (fold, call, raise, etc.).

**Endpoint:** `POST /api/sessions/{session_id}/actions`

**Request Body:**
```json
{
  "action": "Fold"           // Fold
}
```

Or for betting actions:
```json
{
  "action": {"Bet": 200}     // Bet 200 chips
}
```

```json
{
  "action": {"Raise": 400}   // Raise to 400 total
}
```

```json
{
  "action": "Call"           // Call current bet
}
```

```json
{
  "action": "Check"          // Check (no bet)
}
```

```json
{
  "action": "AllIn"          // All-in
}
```

**Response:** `202 Accepted`
```json
{
  "PlayerAction": {
    "player_id": 0,
    "action": {"Bet": 200}
  }
}
```

**Error Response:** `409 Conflict`
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

**Curl Examples:**
```bash
# Fold
curl -X POST http://localhost:8080/api/sessions/{session_id}/actions \
  -H "Content-Type: application/json" \
  -d '{"action": "Fold"}'

# Call
curl -X POST http://localhost:8080/api/sessions/{session_id}/actions \
  -H "Content-Type: application/json" \
  -d '{"action": "Call"}'

# Raise to 500
curl -X POST http://localhost:8080/api/sessions/{session_id}/actions \
  -H "Content-Type: application/json" \
  -d '{"action": {"Raise": 500}}'
```

---

### Delete Session

End and clean up a game session.

**Endpoint:** `DELETE /api/sessions/{session_id}`

**Response:** `204 No Content`

**Curl Example:**
```bash
curl -X DELETE http://localhost:8080/api/sessions/550e8400-e29b-41d4-a716-446655440000
```

---

### Game Lobby (HTML)

Render game lobby with session creation form.

**Endpoint:** `GET /api/game/lobby`

**Response:** `200 OK` (HTML content)

Returns an HTML fragment with htmx-enhanced form for session creation.

---

## Server-Sent Events

### Stream Game Events

Subscribe to real-time game events for a session.

**Endpoint:** `GET /api/sessions/{session_id}/events`

**Content-Type:** `text/event-stream`

**Event Format:**
```
event: game_event
data: {"type":"GameStarted","session_id":"...","players":[...]}

event: game_event
data: {"type":"HandStarted","hand_id":"hand_001","button_player":0}

event: game_event
data: {"type":"CardsDealt","player_id":0,"cards":[...]}

event: game_event
data: {"type":"PlayerAction","player_id":1,"action":"Call"}

event: game_event
data: {"type":"CommunityCards","cards":[...],"street":"Flop"}

event: game_event
data: {"type":"HandCompleted","result":{...}}
```

**Event Types:**

| Event Type | Description | Data Fields |
|------------|-------------|-------------|
| `GameStarted` | Game session initialized | `session_id`, `players` |
| `HandStarted` | New hand begins | `hand_id`, `button_player` |
| `CardsDealt` | Hole cards dealt | `player_id`, `cards` (null for opponent) |
| `CommunityCards` | Board cards revealed | `cards`, `street` |
| `PlayerAction` | Player makes action | `player_id`, `action` |
| `HandCompleted` | Hand ends with result | `result` (winner, pot, etc.) |
| `GameEnded` | Session ends | `winner`, `reason` |
| `Error` | Error occurred | `message` |

**Curl Example:**
```bash
# Stream events (use -N for no buffering)
curl -N http://localhost:8080/api/sessions/550e8400-e29b-41d4-a716-446655440000/events
```

**JavaScript Example:**
```javascript
const eventSource = new EventSource('/api/sessions/' + sessionId + '/events');

eventSource.addEventListener('game_event', (event) => {
  const data = JSON.parse(event.data);
  console.log('Game event:', data);

  // Handle different event types
  switch(data.type) {
    case 'CardsDealt':
      updatePlayerCards(data.player_id, data.cards);
      break;
    case 'CommunityCards':
      updateBoard(data.cards);
      break;
    case 'PlayerAction':
      showAction(data.player_id, data.action);
      break;
    // ...
  }
});

eventSource.onerror = (error) => {
  console.error('SSE error:', error);
  eventSource.close();
};
```

---

## Hand History Endpoints

### Get Recent Hands

Retrieve recent hand history.

**Endpoint:** `GET /api/history?limit=10`

**Query Parameters:**
- `limit` (optional): Number of hands to return (default: 20, max: 100)

**Response:** `200 OK`
```json
{
  "hands": [
    {
      "hand_id": "hand_001",
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": "2025-11-02T10:30:00Z",
      "winner": 0,
      "pot_size": 500,
      "street": "River",
      "hand_type": "Flush"
    }
  ],
  "total": 42,
  "limit": 10
}
```

**Curl Example:**
```bash
curl "http://localhost:8080/api/history?limit=5"
```

---

### Get Hand by ID

Retrieve detailed information for a specific hand.

**Endpoint:** `GET /api/history/{hand_id}`

**Response:** `200 OK`
```json
{
  "hand_id": "hand_001",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-11-02T10:30:00Z",
  "players": [ ... ],
  "actions": [
    {"player_id": 0, "street": "Preflop", "action": "Raise", "amount": 200},
    {"player_id": 1, "street": "Preflop", "action": "Call", "amount": 200}
  ],
  "board": [
    {"rank": "Ace", "suit": "Spades"},
    {"rank": "King", "suit": "Hearts"},
    {"rank": "Queen", "suit": "Diamonds"},
    {"rank": "Jack", "suit": "Clubs"},
    {"rank": "Ten", "suit": "Spades"}
  ],
  "result": {
    "winner": 0,
    "pot_size": 500,
    "hand_type": "Straight",
    "winning_cards": [ ... ]
  }
}
```

**Curl Example:**
```bash
curl http://localhost:8080/api/history/hand_001
```

---

### Filter Hands

Filter hands by various criteria.

**Endpoint:** `POST /api/history/filter`

**Request Body:**
```json
{
  "start_time": "2025-11-01T00:00:00Z",  // Optional
  "end_time": "2025-11-02T23:59:59Z",    // Optional
  "min_pot": 200,                         // Optional
  "max_pot": 1000,                        // Optional
  "winner_id": 0,                         // Optional: filter by winner
  "street": "River",                      // Optional: final street
  "limit": 20,
  "offset": 0
}
```

**Response:** `200 OK`
```json
{
  "hands": [ ... ],
  "total": 15,
  "filters_applied": {
    "min_pot": 200,
    "max_pot": 1000,
    "winner_id": 0
  }
}
```

**Curl Example:**
```bash
curl -X POST http://localhost:8080/api/history/filter \
  -H "Content-Type: application/json" \
  -d '{"min_pot": 300, "limit": 10}'
```

---

### Get Statistics

Retrieve aggregate statistics.

**Endpoint:** `GET /api/history/stats`

**Response:** `200 OK`
```json
{
  "total_hands": 142,
  "player_stats": [
    {
      "player_id": 0,
      "hands_played": 142,
      "hands_won": 78,
      "win_rate": 0.549,
      "total_winnings": 15600,
      "average_pot": 425.3,
      "vpip": 0.68,          // Voluntarily put in pot %
      "pfr": 0.42,           // Pre-flop raise %
      "aggression_factor": 2.1
    },
    {
      "player_id": 1,
      "hands_played": 142,
      "hands_won": 64,
      "win_rate": 0.451,
      "total_winnings": -15600,
      "average_pot": 425.3,
      "vpip": 0.55,
      "pfr": 0.28,
      "aggression_factor": 1.4
    }
  ]
}
```

**Curl Example:**
```bash
curl http://localhost:8080/api/history/stats
```

---

## Settings Endpoints

### Get Settings

Retrieve current application settings.

**Endpoint:** `GET /api/settings`

**Response:** `200 OK`
```json
{
  "default_level": 1,
  "default_opponent": {"AI": "Baseline"},
  "session_timeout_minutes": 30,
  "enable_logging": true,
  "log_level": "info"
}
```

**Curl Example:**
```bash
curl http://localhost:8080/api/settings
```

---

### Update Settings

Update application settings.

**Endpoint:** `PUT /api/settings`

**Request Body:**
```json
{
  "default_level": 2,
  "default_opponent": {"AI": "Aggressive"},
  "session_timeout_minutes": 60
}
```

**Response:** `200 OK`
```json
{
  "default_level": 2,
  "default_opponent": {"AI": "Aggressive"},
  "session_timeout_minutes": 60,
  "enable_logging": true,
  "log_level": "info"
}
```

**Curl Example:**
```bash
curl -X PUT http://localhost:8080/api/settings \
  -H "Content-Type: application/json" \
  -d '{"default_level": 3}'
```

---

### Update Single Setting Field

Update a single setting field (partial update).

**Endpoint:** `PATCH /api/settings/field`

**Request Body:**
```json
{
  "field": "default_level",
  "value": 5
}
```

**Response:** `200 OK`
```json
{
  "field": "default_level",
  "old_value": 2,
  "new_value": 5
}
```

**Curl Example:**
```bash
curl -X PATCH http://localhost:8080/api/settings/field \
  -H "Content-Type: application/json" \
  -d '{"field":"log_level","value":"debug"}'
```

---

### Reset Settings

Reset all settings to defaults.

**Endpoint:** `POST /api/settings/reset`

**Response:** `200 OK`
```json
{
  "default_level": 1,
  "default_opponent": {"AI": "Baseline"},
  "session_timeout_minutes": 30,
  "enable_logging": true,
  "log_level": "info"
}
```

**Curl Example:**
```bash
curl -X POST http://localhost:8080/api/settings/reset
```

---

## Health Check

### Server Health

Check server health status.

**Endpoint:** `GET /health`

**Response:** `200 OK`
```json
{
  "status": "ok"
}
```

**Note:** Health check returns a minimal response. This is a simple endpoint for monitoring service availability.

**Curl Example:**
```bash
curl http://localhost:8080/health
```

---

## Static Assets

Static files are served from the configured static directory.

**Base Path:** `/`

**Examples:**
- `GET /` - Main game interface (index.html)
- `GET /styles.css` - CSS stylesheet
- `GET /app.js` - JavaScript application code
- `GET /assets/cards/AS.svg` - Card images

**Caching:**
Static assets include appropriate cache headers for browser caching.

---

## Rate Limiting

**Current Status:** Not implemented

**Future Consideration:** Rate limiting should be added for production deployments to prevent abuse.

Recommended limits:
- Session creation: 10 per minute per IP
- Action submission: 100 per minute per session
- History queries: 60 per minute per IP

---

## WebSocket Alternative

Currently, the API uses Server-Sent Events (SSE) for real-time updates. SSE is simpler and sufficient for server-to-client streaming.

**Future Consideration:** WebSocket support could be added for bidirectional real-time communication if needed.

---

## API Versioning

**Current Version:** v1 (implicit)

All endpoints are currently unversioned. Future API versions will use URL prefixes:
- v1: `/api/...` (current)
- v2: `/api/v2/...` (future)

---

## CORS Configuration

For local development, CORS is permissive. For production deployments, restrict origins appropriately.

---

## Additional Resources

- [README.md](README.md) - Setup and usage guide
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment configuration
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues
- [../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) - System architecture
