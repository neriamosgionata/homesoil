# Homesoil — Full Project Documentation

> **IoT Home Automation Platform**
> Manage sensors, actuators, and automation workflows through a real-time dashboard.

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture](#2-architecture)
3. [Rust Backend (`homesoil/`)](#3-rust-backend)
4. [SvelteKit Frontend (`homesoil-frontend/`)](#4-sveltekit-frontend)
5. [Android App (`homesoil-android/`)](#5-android-app)
6. [Communication Protocols](#6-communication-protocols)
7. [Database](#7-database)
8. [Authentication & Security](#8-authentication--security)
9. [Automation Engine](#9-automation-engine)
10. [Running Locally](#10-running-locally)
11. [Deployment](#11-deployment)
12. [Test Suites & Coverage](#12-test-suites--coverage)
13. [Test Improvement Plan](#13-test-improvement-plan)

---

## 1. Project Overview

Homesoil is a self-hosted IoT home automation platform composed of three applications:

| Application | Stack | Purpose |
|---|---|---|
| **Backend** | Rust (Axum + socketioxide + CoAP + Diesel/SQLite) | Device management, real-time event hub, automation engine |
| **Frontend** | SvelteKit 5 (Svelte 5 runes, Tailwind CSS, Socket.IO) | Web dashboard for monitoring and control |
| **Android App** | Kotlin (Jetpack Compose, Socket.IO, MVVM) | Mobile dashboard with device scanning |

### Core Capabilities

- **Sensor Management** — Register, monitor, and query historical data from CoAP sensors (temperature, humidity, pressure, current, wind, rain, UV, solar radiation).
- **Actuator Control** — Toggle, pulse, and intermittent (cyclic ON/OFF) control of CoAP actuators.
- **Visual Flow Automation** — Node-based visual editor (XYFlow) for creating sensor→logic→actuator automation graphs.
- **Script Automation** — Custom DSL for imperative automation scripts with scheduling (cron).
- **Real-Time Communication** — Socket.IO (WebSocket) for instant UI updates across all connected clients.
- **Device Discovery** — CoAP multicast for automatic device registration on the local network.

### Repository Layout

```
Homesoil/
├── homesoil/              # Rust backend
├── homesoil-frontend/     # SvelteKit 5 frontend
├── homesoil-android/      # Android app (Kotlin/Compose)
├── fake-actuator/         # CoAP actuator emulator (dev tool)
├── fake-sensor/           # CoAP sensor emulator (dev tool)
└── DOCUMENTATION.md       # This file
```

---

## 2. Architecture

### System Architecture

```
┌──────────────────┐     ┌──────────────────┐
│   Web Browser    │     │   Android App    │
│  (SvelteKit 5)   │     │  (Jetpack Comp.) │
└────────┬─────────┘     └────────┬─────────┘
         │ Socket.IO (WS/WSS)     │ Socket.IO (WS/WSS)
         └───────────┬────────────┘
                     │
              ┌──────▼──────────────────────────────┐
              │         Rust Backend                  │
              │                                       │
              │  ┌──────────┐   ┌──────────────────┐ │
              │  │ Axum HTTP │   │ Socket.IO Engine │ │
              │  │  :4000    │   │  (socketioxide)  │ │
              │  └──────────┘   └──────────────────┘ │
              │                                       │
              │  ┌──────────────────────────────────┐ │
              │  │       Event System (30+ events)  │ │
              │  │  Sensors · Actuators · Flows ·   │ │
              │  │  Scripts · Auth · Messages        │ │
              │  └──────────────────────────────────┘ │
              │                                       │
              │  ┌──────────┐   ┌──────────────────┐ │
              │  │   CoAP   │   │  Flow Engine +   │ │
              │  │  Server   │   │  Script Parser   │ │
              │  │:5683/DTLS │   │                  │ │
              │  └──────────┘   └──────────────────┘ │
              │                                       │
              │  ┌──────────────────────────────────┐ │
              │  │    SQLite (Diesel ORM)           │ │
              │  └──────────────────────────────────┘ │
              └──────────────┬──────────────────────┘
                             │ CoAP (UDP / DTLS)
              ┌──────────────┼──────────────┐
              │              │              │
       ┌──────▼──────┐ ┌────▼─────┐ ┌──────▼──────┐
       │  Sensor 1   │ │ Sensor N │ │ Actuator 1  │ ...
       │  (CoAP)     │ │  (CoAP)  │ │   (CoAP)    │
       └─────────────┘ └──────────┘ └─────────────┘
```

### Threading Model

```
Main Thread (tokio runtime)
├── Task: Socket.IO server (Axum on :4000)
│   └── Handles WebSocket upgrades, event routing
├── Task: CoAP server (UDP on :5683)
│   └── Device registration, sensor reads, actuator commands
├── Thread: Sensor health check (every 5s)
│   └── Pings all devices, updates online status
├── Thread: Intermittent actuator state machine (per-actuator)
│   └── Cyclic ON/OFF with configurable intervals
└── Thread: Sensor read cleanup (hourly)
    └── Deletes records older than 30 days
```

### Data Flow — User Toggles Actuator

```
1. Dashboard emits: socket.emit('toggle-actuator', { actuator_id: 5 })
2. Backend handler fetches actuator from SQLite
3. Backend sends CoAP POST to coap://<device_ip>:<port>/actuator/state
4. Device responds, backend updates DB: state = true
5. Backend broadcasts: socket.broadcast('actuator-state-change', { id: 5, state: true })
6. All connected dashboards update instantly
```

---

## 3. Rust Backend

### 3.1 Directory Structure

```
homesoil/
├── Cargo.toml
├── diesel.toml
├── deploy.sh
├── docker-compose.yml
├── .env / .env.example
├── src/
│   ├── main.rs                # Entry point (server startup)
│   ├── lib.rs                 # Module declarations
│   ├── db.rs                  # SQLite connection
│   ├── auth.rs                # PIN/token auth + rate limiting
│   ├── models.rs              # Diesel ORM models (~1,091 lines)
│   ├── schema.rs              # Auto-generated Diesel schema
│   ├── servers.rs             # Socket.IO + health check setup
│   ├── server.rs              # CoAP server implementation
│   ├── client.rs              # CoAP client (device commands)
│   ├── coap_client.rs         # Centralized CoAP command sender (DTLS-aware)
│   ├── dtls.rs                # DTLS-PSK server/client wrappers
│   ├── observer.rs            # CoAP observe pattern
│   ├── message/mod.rs         # CoAP message codec
│   ├── handlers.rs            # CoAP request routing
│   ├── validation.rs          # Input validation (IP, port, payload size, sensor values)
│   ├── events.rs              # Socket.IO event system (~1,030 lines)
│   ├── sensor_methods.rs      # Sensor CRUD
│   ├── sensor_handlers.rs     # Sensor event→CoAP bridge
│   ├── sensor_types.rs        # Sensor type constants
│   ├── actuator_methods.rs    # Actuator CRUD
│   ├── actuator_handlers.rs   # Actuator event→CoAP bridge
│   ├── flow_engine.rs         # Visual flow evaluation engine
│   ├── flow_methods.rs        # Flow CRUD
│   ├── script_parser.rs       # Custom DSL parser (~1,104 lines)
│   ├── script_methods.rs      # Script CRUD
│   ├── script_runner.rs       # Script executor (stub)
│   ├── condition_parser.rs    # IF condition evaluator
│   ├── intermittent.rs        # Cyclic actuator control
│   └── helper.rs              # UI notification helper
├── migrations/                # Diesel migrations (7 total)
├── tests/                     # Integration tests
│   ├── auth_tests.rs
│   ├── condition_parser_tests.rs
│   ├── flow_engine_tests.rs
│   └── flow_methods_tests.rs
└── db/                        # SQLite database file
```

**Total: ~8,350 lines of Rust**

### 3.2 Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `tokio` | 1 (full) | Async runtime |
| `axum` | 0.6.20 | HTTP framework |
| `socketioxide` | 0.8.0 | Socket.IO v5 server |
| `diesel` | 2.1.4 (sqlite, chrono) | ORM |
| `coap-lite` | 0.11.3 | CoAP protocol |
| `serde` / `serde_json` | 1.x | Serialization |
| `chrono` | 0.4.31 | Date/time handling |
| `anyhow` | 1.0.75 | Error handling |
| `rand` | 0.8.5 | Random generation |
| `regex` | 1.10.2 | Pattern matching |
| `lru_time_cache` | 0.11.11 | Block transfer cache |
| `local-ip-address` | 0.5.6 | Network IP detection |
| `openssl` | 0.10 | DTLS-PSK for CoAP encryption |
| `sha2` | 0.10 | SHA-256 session token hashing |
| `axum-server` | 0.5 (tls-rustls) | TLS support for Socket.IO |
| `env_logger` | 0.11 | Structured logging |

### 3.3 Startup Sequence

1. Initialize structured logging (`env_logger`)
2. Load `.env` (DATABASE_URL, IS_DEV)
3. Connect to SQLite database
4. Generate device credentials: 8-digit PIN + 32-char device secret
5. Set PIN creation timestamp (PIN expires after 10 minutes)
6. Print credentials to console
7. Start Socket.IO server on `:4000` (with TLS if `TLS_CERT_PATH`/`TLS_KEY_PATH` set)
8. Start sensor health check loop (5s interval)
9. Start CoAP server on `:5683`
10. Start sensor read cleanup task (1h interval)
11. Start expired session token cleanup thread (1h interval)

### 3.4 Socket.IO Events

#### Server → Client

| Event | Payload | When |
|---|---|---|
| `all-sensors` | `{ sensors: Sensor[] }` | On connect |
| `all-actuators` | `{ actuators: Actuator[] }` | On connect |
| `all-last-sensors-reads` | `{ sensor_reads }` | On connect |
| `all-scripts` | `{ scripts: Script[] }` | On connect |
| `all-flows` | `{ flows: Flow[] }` | On connect |
| `sensor-register` | `{ sensor }` | Device connected |
| `sensor-unregister` | `{ sensor_id }` | Device disconnected |
| `sensor-read` | `{ sensor_id, value }` | New reading |
| `sensor-online-status-change` | `{ sensor_id, online }` | Health check |
| `actuator-register` | `{ actuator }` | Device connected |
| `actuator-unregister` | `{ actuator_id }` | Device disconnected |
| `actuator-state-change` | `{ actuator_id, state }` | Toggle/pulse |
| `flow-saved` / `flow-modified` | `{ flow }` | Flow CRUD |
| `script-saved` / `script-deleted` | `{ script }` | Script CRUD |
| `message-sent` | `{ message, type }` | UI notification |

#### Client → Server

| Event | Payload | Action |
|---|---|---|
| `toggle-actuator` | `{ actuator_id }` | Toggle ON/OFF |
| `pulse-actuator` | `{ actuator_id }` | Momentary pulse |
| `intermittent-actuator` | `{ id, on_ms, off_ms }` | Start cyclic |
| `stop-intermittent-actuator` | `{ actuator_id }` | Stop cyclic |
| `get-sensor-readings` | `{ id, from_date, to_date }` | Query history (limit 50) |
| `sensor-name-change` | `{ sensor_id, name }` | Rename |
| `actuator-name-change` | `{ actuator_id, name }` | Rename |
| `add-flow` / `modify-flow` / `remove-flow` | `{ flow }` | Flow CRUD |
| `add-script` / `modify-script` / `run-script` | `{ script }` | Script CRUD |

### 3.5 CoAP Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/sensor/register` | Register new sensor |
| `POST` | `/sensor/unregister` | Remove sensor |
| `PUT` | `/sensor/name` | Rename sensor |
| `POST` | `/sensor` | Submit sensor reading |
| `POST` | `/actuator/register` | Register new actuator |
| `POST` | `/actuator/unregister` | Remove actuator |
| `PUT` | `/actuator/name` | Rename actuator |
| `PUT` | `/actuator/state` | Change actuator state |
| `GET` | `/.well-known/core` | Device discovery (multicast) |

All CoAP requests must include `device_secret` in the JSON payload for authentication.

**CoAP Response Codes:**
- `2.05 Content` — Successful operation
- `4.00 Bad Request` — Malformed or unparseable payload
- `4.01 Unauthorized` — Invalid or missing `device_secret`
- `4.04 Not Found` — Unknown endpoint path

**DTLS Support:** CoAP traffic can be encrypted with DTLS-PSK (Pre-Shared Key). The `device_secret` serves as the PSK. DTLS configuration is in `dtls.rs`.

### 3.6 Module Dependency Graph

```
main.rs
├── servers.rs ← auth.rs, events.rs, handlers.rs, flow_engine.rs, intermittent.rs
├── server.rs ← message/mod.rs, observer.rs
├── coap_client.rs ← dtls.rs, client.rs (centralized CoAP command sender)
├── dtls.rs ← openssl (DTLS-PSK transport)
├── validation.rs (input validation for CoAP payloads)
├── events.rs ← sensor_methods/handlers, actuator_methods/handlers,
│                script_methods, flow_methods, script_parser,
│                condition_parser, helper, intermittent, coap_client
├── flow_engine.rs ← flow_methods, actuator_methods, sensor_methods, coap_client
├── db.rs ← schema.rs
└── models.rs ← schema.rs
```

---

## 4. SvelteKit Frontend

### 4.1 Directory Structure

```
homesoil-frontend/
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
├── src/
│   ├── app.css                  # Global styles + CSS variables
│   ├── app.html                 # HTML shell
│   ├── app.d.ts                 # SvelteKit types
│   ├── test-setup.ts            # Vitest configuration
│   ├── routes/
│   │   ├── +page.svelte         # Login page (PIN/token)
│   │   ├── +page.server.ts      # SSR disabled
│   │   ├── +layout.svelte       # Root layout
│   │   └── dashboard/
│   │       ├── +layout.svelte   # Dashboard shell + WS connection
│   │       ├── +page.svelte     # Main dashboard (sensors + actuators)
│   │       ├── sensors/[id]/+page.svelte   # Sensor detail + chart
│   │       ├── flows/
│   │       │   ├── +page.svelte            # Flow list
│   │       │   ├── new/+page.svelte        # Create flow
│   │       │   └── [id]/+page.svelte       # Edit flow
│   │       └── scripts/
│   │           ├── +page.svelte            # Script list
│   │           ├── new/+page.svelte        # Create script
│   │           └── [id]/+page.svelte       # Edit script
│   └── lib/
│       ├── components/
│       │   ├── Sidebar.svelte
│       │   ├── DashboardMessage.svelte
│       │   ├── Sensor.svelte
│       │   ├── Actuator.svelte
│       │   ├── ProgressBar.svelte
│       │   ├── FlowEditor.svelte
│       │   ├── FlowCard.svelte
│       │   ├── SensorInputNode.svelte
│       │   ├── ActuatorOutputNode.svelte
│       │   ├── ComparisonNode.svelte
│       │   ├── LogicGateNode.svelte
│       │   ├── ConstantNode.svelte
│       │   ├── ScriptEditor.svelte
│       │   ├── Script.svelte
│       │   └── CronInput.svelte
│       ├── models/              # TypeScript interfaces
│       ├── stores/store.ts      # Svelte writable stores
│       ├── websocket/
│       │   ├── Websocket.ts     # Socket.IO client wrapper
│       │   ├── WebsocketListenEventMap.ts
│       │   └── WebsocketEmitEventMap.ts
│       ├── enums/               # Event enums
│       ├── parser/              # Sensor value formatting
│       └── index.ts
└── static/
    └── favicon.png
```

### 4.2 Key Dependencies

| Package | Version | Purpose |
|---|---|---|
| `svelte` | 5.53.12 | UI framework (runes syntax) |
| `@sveltejs/kit` | 2.55.0 | App framework |
| `vite` | 8.0.0 | Build tool |
| `socket.io-client` | 4.8.3 | Real-time communication |
| `@xyflow/svelte` | 1.5.1 | Visual flow editor |
| `flowbite-svelte` | 1.31.0 | UI component library |
| `tailwindcss` | 4.2.1 | Utility CSS |
| `@tabler/icons-svelte` | 3.40.0 | Icon set |
| `moment` | 2.30.1 | Date formatting |
| `dompurify` | 3.3.3 | XSS sanitization |
| `vitest` | 4.1.0 | Test framework |

### 4.3 State Management

All state is managed through Svelte writable stores (dictionary-keyed by ID for O(1) lookups):

```typescript
sensors:            { [id: number]: Sensor }
actuators:          { [id: number]: Actuator }
last_sensor_reads:  { [id: number]: SensorRead }
sensor_reads:       SensorRead[]
scripts:            { [id: number]: Script }
flows:              { [id: number]: Flow }
dashboard_message:  DashboardMessage | null
sensor_reads_loading: boolean
socket_token:       { token: string; pin?: string }   // persisted to localStorage
```

**Reactivity pattern**: Store updates create new object references via spread (`{ ...obj, field: newValue }`) to trigger Svelte 5 `$derived`/`$props` updates. Mutation-in-place does **not** work.

### 4.4 WebSocket Integration

The `Websocket` class wraps `socket.io-client` and:

- Connects to `${hostname}:4000` with token or PIN auth
- Registers 30+ listen event handlers that update Svelte stores
- Exposes typed emit methods for all client→server events
- Is provided via Svelte `setContext` to avoid prop drilling

### 4.5 Routing

All pages are client-rendered (`export const ssr = false`). The dashboard layout establishes the WebSocket connection and provides it to all child routes.

| Route | Page |
|---|---|
| `/` | Login (PIN entry or token reconnect) |
| `/dashboard` | Main dashboard (sensor cards + actuator cards) |
| `/dashboard/sensors/[id]` | Sensor detail with historical chart |
| `/dashboard/flows` | Flow list |
| `/dashboard/flows/new` | Create new visual flow |
| `/dashboard/flows/[id]` | Edit existing flow |
| `/dashboard/scripts` | Script list |
| `/dashboard/scripts/new` | Create new script |
| `/dashboard/scripts/[id]` | Edit existing script |

### 4.6 Styling

- **Theme**: Dark mode with CSS custom properties (`--bg-primary: #0f172a`, accent green `#4CAF50`)
- **Effects**: Glass morphism (`.glass` — backdrop blur + subtle border)
- **Layout**: Responsive grid (1→2→3 columns)
- **Tailwind v4** with forms, typography, aspect-ratio, container-queries plugins

### 4.7 TypeScript Models

```typescript
interface Sensor {
  id: number; name: string; sensor_type: SensorTypeEnum;
  ip_address: string; port: number; online: boolean;
  created_at: string; updated_at: string | null;
}

interface Actuator {
  id: number; name: string; ip_address: string; port: number;
  state: boolean; pulse: boolean; online: boolean;
  intermittent: boolean; intermittent_on_ms: number; intermittent_off_ms: number;
  created_at: string; updated_at: string | null;
}

interface Flow {
  id: number; title: string; graph: string; // JSON { nodes[], edges[] }
  enabled: boolean; created_at: string; updated_at: string | null;
}

interface Script {
  id: number; title: string; code: string;
  schedule: string | null; status: number; // 0=stopped, 1=running, 2=error
  created_at: string; updated_at: string | null;
}

interface SensorRead {
  id: number; sensor_id: number; sensor_value: string;
  created_at: string; updated_at: string | null;
}

enum SensorTypeEnum {
  current, temperature, humidity, pressure,
  wind_speed, wind_direction, rain, uv, solar_radiation, unknown
}
```

---

## 5. Android App

### 5.1 Directory Structure

```
homesoil-android/
├── build.gradle.kts
├── settings.gradle.kts
├── app/
│   ├── build.gradle.kts
│   └── src/main/
│       ├── AndroidManifest.xml
│       └── java/com/homesoil/app/
│           ├── HomesoilApp.kt              # Application class
│           ├── MainActivity.kt             # Single Activity entry
│           ├── data/
│           │   ├── models/                 # 9 data classes
│           │   └── repository/
│           │       └── HomesoilRepository.kt
│           ├── network/
│           │   ├── SocketManager.kt        # Socket.IO client (~855 lines)
│           │   └── SocketEvents.kt         # Event constants
│           ├── ui/
│           │   ├── components/             # 5 reusable composables
│           │   ├── navigation/NavGraph.kt  # Navigation routes
│           │   ├── screens/               # 8 screens + ViewModels
│           │   └── theme/                 # Material3 theme
│           └── util/
│               └── DateUtils.kt
```

**Total: ~8,500 lines of Kotlin**

### 5.2 Architecture: MVVM + Reactive Data Flow

```
┌────────────┐     ┌──────────────┐     ┌──────────────┐     ┌─────────────┐
│  UI Layer  │ ──► │  ViewModel   │ ──► │  Repository  │ ──► │SocketManager│
│ (Compose)  │ ◄── │  (StateFlow) │ ◄── │  (Facade)    │ ◄── │ (Socket.IO) │
└────────────┘     └──────────────┘     └──────────────┘     └─────────────┘
```

- **Unidirectional data flow** with Kotlin `StateFlow` / `SharedFlow`
- **Single Activity** with Jetpack Compose Navigation
- **No Fragments** — pure Compose screens

### 5.3 Build Configuration

| Setting | Value |
|---|---|
| Target SDK | 36 (Android 15) |
| Min SDK | 26 (Android 8) |
| Compile SDK | 36 |
| Kotlin JVM Target | 11 |
| Compose | Enabled |

### 5.4 Key Dependencies

- **Jetpack Compose** — Material3, extended icons, activity-compose
- **Lifecycle** — ViewModel, runtime (compose-aware)
- **Navigation** — Compose navigation
- **DataStore** — Encrypted preferences for token storage
- **Socket.IO** — `io.socket:socket.io-client`
- **Serialization** — `kotlinx.serialization.json`
- **Charts** — Vico charting library (sensor history)
- **Coroutines** — Kotlin coroutines for async

### 5.5 Screens

| Screen | ViewModel | Purpose |
|---|---|---|
| `LoginScreen` | `LoginViewModel` | Server discovery (subnet scan), PIN/token auth |
| `DashboardScreen` | `DashboardViewModel` | Tabbed view: Sensors + Actuators |
| `SensorDetailScreen` | `SensorDetailViewModel` | Historical chart, date range, rename/delete |
| `ScriptsScreen` | `ScriptsViewModel` | Script list with run/delete actions |
| `ScriptEditorScreen` | `ScriptEditorViewModel` | Code editor + cron scheduling |
| `FlowsScreen` | `FlowsViewModel` | Flow list with enable/disable toggle |
| `FlowEditorScreen` | `FlowEditorViewModel` | Visual node-based flow builder |
| `SettingsScreen` | `SettingsViewModel` | Server config, connection test |

### 5.6 Reusable Components

- `SensorCard` — Sensor value display with online indicator and type-specific icon/color
- `ActuatorCard` — Toggle/pulse/intermittent controls with rename dialog
- `ScriptCard` — Script preview with run button and status indicator
- `FlowCard` — Flow card with enable/disable toggle
- `CronInput` — Cron expression builder with preset options

### 5.7 Networking

`SocketManager` handles the entire Socket.IO lifecycle:

- WebSocket + polling fallback transport
- Token-based auth via connection header
- Reconnection: 5 attempts, 1s delay, 10s timeout
- 45+ event handlers organized by domain (sensors, actuators, scripts, flows)
- State exposed as `StateFlow<Map<Int, Model>>` for O(1) lookups
- JSON parsing via `kotlinx.serialization` with `ignoreUnknownKeys = true`

### 5.8 Authentication Flow

1. **Server Discovery** — Scans subnet (1-254) in parallel for backend on configured port
2. **Token Auth** — Token stored in DataStore preferences, passed in Socket.IO auth header
3. **Session Persistence** — Host, port, token saved and auto-loaded on restart

### 5.9 Data Models

```kotlin
@Serializable data class Sensor(id: Int, name: String, sensorType: SensorType, ...)
@Serializable data class Actuator(id: Int, name: String, state: Boolean, pulse: Boolean, intermittent: Boolean, ...)
@Serializable data class SensorRead(id: Int, sensorId: Int, sensorValue: String, ...)
@Serializable data class Script(id: Int, title: String, code: String, schedule: String?, status: Int, ...)
@Serializable data class DeviceFlow(id: Int, title: String, graph: String, enabled: Boolean, ...)
@Serializable data class FlowGraph(nodes: List<FlowNode>, edges: List<FlowEdge>)

enum class SensorType { TEMPERATURE, HUMIDITY, PRESSURE, CURRENT, WIND_SPEED, WIND_DIRECTION, RAIN, UV, SOLAR_RADIATION, UNKNOWN }
enum class FlowNodeType { SENSOR_INPUT, ACTUATOR_OUTPUT, COMPARISON, LOGIC_GATE, CONSTANT }
```

---

## 6. Communication Protocols

### 6.1 Socket.IO (Clients ↔ Backend)

- **Protocol**: Socket.IO v5 (socketioxide 0.8 on server, socket.io-client 4.8 on clients)
- **Transport**: WebSocket with HTTP long-polling fallback; TLS optional (via `TLS_CERT_PATH`/`TLS_KEY_PATH`)
- **Port**: 4000 (HTTP or HTTPS depending on TLS config)
- **Auth**: Token or PIN sent in connection handshake; tokens stored as SHA-256 hashes in DB
- **Pattern**: Event-driven — clients emit commands, server broadcasts state changes to all

### 6.2 CoAP (Backend ↔ IoT Devices)

- **Protocol**: CoAP 1.0 (RFC 7252)
- **Transport**: UDP (plain) or DTLS 1.2 (encrypted, PSK cipher suites)
- **Port**: 5683 (standard CoAP), 5684 (CoAPS/DTLS)
- **Features implemented**:
  - Confirmable (CON) and Non-Confirmable (NON) messages
  - Block-wise transfers (Block1 upload, Block2 download, 1KB chunks)
  - Observe pattern (pub/sub for resource updates)
  - Multicast discovery (IPv4 224.0.1.187, IPv6 ff0x::fd) — rate limited (5/min per source IP), disableable via `DISABLE_DISCOVERY=true`
  - DTLS-PSK encryption (`dtls.rs`) — `device_secret` as pre-shared key
  - Proper CoAP response codes (4.01 Unauthorized, 4.00 Bad Request, 4.04 Not Found)
- **Auth**: `device_secret` field in every JSON payload
- **Input validation**: IP address format, payload size (max 1KB), sensor value parsing

### 6.3 Health Check Protocol

Every 5 seconds, the backend:
1. Sends CoAP GET to each registered sensor and actuator
2. Updates `online` status in SQLite based on response
3. Broadcasts `sensor-online-status-change` / `actuator-online-status-change` to all Socket.IO clients

---

## 7. Database

### 7.1 Engine

**SQLite** via Diesel ORM 2.1.4. Single-file database stored in `homesoil/db/`.

### 7.2 Schema

```sql
-- sensors
CREATE TABLE sensors (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT DEFAULT 'Default',
    sensor_type TEXT NOT NULL,
    ip_address  TEXT NOT NULL,
    port        SMALLINT NOT NULL,
    online      TINYINT NOT NULL DEFAULT 0,
    dtls_supported BOOLEAN NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME
);
CREATE INDEX idx_sensor_type ON sensors(sensor_type);

-- sensor_reads
CREATE TABLE sensor_reads (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id    INTEGER NOT NULL REFERENCES sensors(id),
    sensor_value TEXT NOT NULL,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME
);

-- actuators
CREATE TABLE actuators (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT,
    ip_address         TEXT NOT NULL,
    port               SMALLINT NOT NULL,
    state              TINYINT NOT NULL DEFAULT 0,
    online             TINYINT NOT NULL DEFAULT 0,
    pulse              TINYINT NOT NULL DEFAULT 0,
    intermittent       TINYINT NOT NULL DEFAULT 0,
    intermittent_on_ms  INTEGER NOT NULL DEFAULT 1000,
    intermittent_off_ms INTEGER NOT NULL DEFAULT 1000,
    dtls_supported     BOOLEAN NOT NULL DEFAULT 0,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         DATETIME
);

-- flows
CREATE TABLE flows (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL DEFAULT 'Default',
    graph      TEXT NOT NULL DEFAULT '{}',
    enabled    TINYINT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);

-- scripts
CREATE TABLE scripts (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL,
    code       TEXT NOT NULL,
    schedule   TEXT,
    status     INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);

-- session_tokens
CREATE TABLE session_tokens (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    token      TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 7.3 Entity Relationships

```
Sensors (1) ──◄ (Many) SensorReads
Actuators          (standalone)
Flows              (standalone — graph JSON embeds node/edge references)
Scripts            (standalone)
SessionTokens      (standalone)
```

### 7.4 Data Retention

| Data | Retention | Mechanism |
|---|---|---|
| Sensor reads | 30 days | Hourly cleanup task |
| Session tokens | 30 days | Deleted on failed validation |
| Sensors/Actuators | Permanent | Manual unregister only |
| Flows/Scripts | Permanent | Manual deletion only |

### 7.5 Migrations

| # | Date | Migration |
|---|---|---|
| 1 | 2023-12-12 | Create `sensors` table |
| 2 | 2023-12-13 | Create `sensor_reads` table |
| 3 | 2023-12-18 | Create `actuators` table |
| 4 | 2024-01-18 | Create `scripts` table |
| 5 | 2026-03-17 | Create `flows` table |
| 6 | 2026-03-25 | Create `session_tokens` table |
| 7 | 2026-03-25 | Add `intermittent_*` columns to `actuators` |
| 8 | 2026-04-07 | Security hardening: add `dtls_supported` to sensors/actuators, clear session_tokens |

Run migrations: `cd homesoil && diesel migration run`

---

## 8. Authentication & Security

### 8.1 Authentication Flow

```
┌─────────────────────────────────────────────────────────────┐
│  First-time Pairing                                          │
│                                                              │
│  1. Backend generates 8-digit PIN + 32-char device secret   │
│  2. PIN displayed in console (expires after 10 minutes)     │
│  3. Client sends { pin: "12345678" } on Socket.IO connect   │
│  4. Backend validates PIN (max 3 attempts, exponential       │
│     backoff: 60s → 300s → 3600s lockout)                    │
│  5. Backend generates 48-char session token, stores          │
│     SHA-256 hash in DB                                       │
│  6. Client receives plaintext token, stores in localStorage/ │
│     DataStore                                                │
│                                                              │
│  Subsequent Connections                                      │
│                                                              │
│  1. Client sends { token: "abc..." } on Socket.IO connect   │
│  2. Backend hashes token with SHA-256, looks up hash in DB  │
│  3. Expired tokens (>30 days) deleted by hourly cleanup     │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 Rate Limiting

| Control | Limit | Details |
|---|---|---|
| **PIN attempts** | 3 per socket | Exponential backoff: 60s → 300s → 3600s lockout |
| **PIN expiry** | 10 minutes | PIN becomes invalid after startup timeout |
| **CoAP discovery** | 5 per source IP per 60s | Prevents network mapping/abuse |
| **Actuator toggle** | 1 per actuator per second | Prevents rapid-fire state changes |
| **Actuator pulse** | 1 per actuator per 3 seconds | Protects physical devices from damage |

### 8.3 Transport Security

| Channel | Encryption | Configuration |
|---|---|---|
| **Frontend ↔ Backend** | TLS (optional) | Set `TLS_CERT_PATH` + `TLS_KEY_PATH` env vars |
| **Devices ↔ Backend** | DTLS-PSK (optional) | `dtls.rs` — `device_secret` as PSK |
| **Multicast Discovery** | None | Rate limited; disable with `DISABLE_DISCOVERY=true` |

- **TLS**: Axum server binds with `axum_server::bind_rustls()` when certificates are provided. Falls back to plain HTTP with a startup warning otherwise.
- **DTLS**: Pre-Shared Key mode using OpenSSL. Cipher suites: `PSK-AES256-CBC-SHA:PSK-AES128-CBC-SHA`. The existing `device_secret` doubles as the PSK.
- **Frontend auto-detection**: `Websocket.ts` detects `https://` page protocol and switches to `wss://` automatically.

### 8.4 CoAP Device Auth

All CoAP requests include `device_secret` in the JSON payload. The backend validates it and returns proper CoAP response codes:
- `4.01 Unauthorized` for invalid/missing `device_secret`
- `4.00 Bad Request` for unparseable payloads
- `4.04 Not Found` for unknown paths

### 8.5 Input Validation (`validation.rs`)

| Validation | Rule | Applied To |
|---|---|---|
| IP address | Must be valid IPv4/IPv6; rejects loopback unless `IS_DEV=true` | Sensor/actuator registration |
| Port | Range 1024–32767 | Sensor/actuator registration |
| Sensor value | Must parse as `f64` | Sensor readings |
| Payload size | Max 1024 bytes | All CoAP registration and read payloads |

### 8.6 Script Engine Resource Limits

| Limit | Value | Protects Against |
|---|---|---|
| Max execution time | 5 minutes | Infinite scripts blocking threads |
| Max loop iterations | 10,000 per LOOP/WHILE | Unbounded loops |
| Max DELAY per call | 60 seconds | Extremely long delays |
| Max variables | 100 | Memory exhaustion |

### 8.7 Session Token Security

- Tokens are **hashed with SHA-256** before storage in the `session_tokens` table
- Incoming tokens are hashed before DB lookup — plaintext tokens never touch the database
- Expired tokens (>30 days) cleaned up by a background thread every hour

### 8.8 Structured Logging

All `println!()` calls replaced with `env_logger` / `log` crate macros:
- `info!` — normal operations (startup, connections, disconnections)
- `warn!` — security events (failed auth, rate limits, rejected CoAP requests)
- `error!` — failures (database errors, CoAP timeouts)
- Secrets, tokens, and full payloads are **never logged**

### 8.9 Security Posture Summary

| Area | Status | Notes |
|---|---|---|
| Socket.IO transport | TLS ready | Enabled via env vars; auto-detected by frontend |
| CoAP transport | DTLS-PSK ready | `dtls.rs` module; `device_secret` as PSK |
| PIN brute force | Hardened | 8-digit PIN, 3 attempts, exponential backoff, 10-min expiry |
| Session tokens | SHA-256 hashed | Plaintext tokens never stored in DB |
| Token storage (web) | localStorage | Cleared only on explicit logout |
| Token storage (Android) | DataStore | Encrypted at rest by Android |
| Input validation | Comprehensive | IP, port, payload size, sensor values validated |
| CoAP response codes | Proper | 4.01, 4.00, 4.04 returned (not plaintext strings) |
| Script engine | Resource-limited | Timeout, loop cap, delay cap, variable cap |
| Actuator commands | Rate limited | 1/sec toggle, 1/3sec pulse per actuator |
| Discovery | Rate limited | 5/min per source IP; can be fully disabled |
| Logging | Structured | `env_logger` with severity levels; no secret leakage |
| XSS protection | DOMPurify | Used in frontend for user-generated content |

---

## 9. Automation Engine

### 9.1 Visual Flows

Flows are node-based automation graphs built with XYFlow. Each flow consists of:

**Node Types:**

| Node | Inputs | Output | Purpose |
|---|---|---|---|
| `SensorInput` | — | `Number` | Live sensor reading as trigger |
| `Constant` | — | `Number` | Fixed value |
| `Comparison` | 2 values | `Boolean` | `>`, `<`, `>=`, `<=`, `==`, `!=` |
| `LogicGate` | 1-2 booleans | `Boolean` | `AND`, `OR`, `NOT` |
| `ActuatorOutput` | 1 boolean | — | Activates/deactivates actuator |

**Evaluation:**
1. Graph is topologically sorted (cycle detection)
2. Nodes evaluated in dependency order
3. Sensor input nodes match against the triggering sensor
4. Values propagate through edges
5. Actuator output nodes send CoAP commands based on final boolean

**Trigger:** Flows are evaluated on every sensor reading received.

### 9.2 Scripts (DSL)

A custom scripting language parsed by `script_parser.rs` (~1,104 lines):

**Available Functions:**
- `ACTIVATE <actuator_id>` — Turn actuator ON
- `DEACTIVATE <actuator_id>` — Turn actuator OFF
- `PULSE <actuator_id>` — Momentary pulse
- `READ <sensor_id>` — Get current sensor value
- `SEND_TO_DASHBOARD <message>` — Show notification
- `SET <var> <value>` / `UNSET <var>` — Variable management
- `ADD`, `SUBTRACT`, `MULTIPLY`, `DIVIDE`, `MODULO` — Arithmetic
- `DELAY <ms>` — Wait (capped at 60,000ms per call)

**Control Flow:**
- `IF <condition> THEN ... END` — Conditional
- `WHILE <condition> ... END` — Loop (max 10,000 iterations)
- `LOOP <n> ... END` — Fixed iterations (max 10,000 iterations)
- `BREAK` / `CONTINUE` — Loop control

**Resource Limits:** Scripts are killed if they exceed 5 minutes of execution, 10,000 loop iterations, 60s per DELAY, or 100 variables. Errors are reported to the dashboard.

**Scheduling:** Scripts can have a cron expression for recurring execution.

### 9.3 Condition Parser

Evaluates conditions in both flows and scripts:

- **Operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`
- **Types**: Integer, float, string, boolean
- **Variables**: `$variable_name` substitution
- **Logic**: AND, OR operators for compound conditions

---

## 10. Running Locally

### 10.1 Prerequisites

- **Rust** (stable) + `cargo`
- **Bun** (or Node.js) for frontend
- **diesel_cli** (`cargo install diesel_cli --no-default-features --features sqlite`)
- **Android Studio** (for Android app)

### 10.2 Backend

```bash
cd homesoil

# Create .env file
cat > .env << 'EOF'
DATABASE_URL=sqlite:./db/homesoil.sqlite
IS_DEV=true
EOF

# Run migrations
diesel migration run

# Start backend
cargo run --release
# → Socket.IO on http://127.0.0.1:4000
# → CoAP on udp://0.0.0.0:5683
# → Console shows PIN and device secret
```

### 10.3 Frontend

```bash
cd homesoil-frontend
bun install
bun run dev
# → http://localhost:5173
```

### 10.4 Android

Open `homesoil-android/` in Android Studio, build and run on device or emulator. Configure server IP and port in the login screen.

### 10.5 Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | SQLite database path (`sqlite:./db/homesoil.sqlite`) |
| `IS_DEV` | No | `false` | If `true`, binds to `127.0.0.1`; allows loopback IPs in validation |
| `LOGIN_TOKEN` | No | — | Override auth token (if set) |
| `COAP_PORT` | No | `5683` | CoAP server port |
| `TLS_CERT_PATH` | No | — | PEM certificate file path for HTTPS/WSS |
| `TLS_KEY_PATH` | No | — | PEM private key file path for HTTPS/WSS |
| `DISABLE_DISCOVERY` | No | `false` | Set `true` to disable CoAP multicast discovery |
| `RUST_LOG` | No | `info` | Log level filter (e.g., `debug`, `warn`, `homesoil=debug`) |

---

## 11. Deployment

### 11.1 Docker

A `docker-compose.yml` is provided in the `homesoil/` directory:
- Exposes ports 4000 (Socket.IO) and 5683/udp (CoAP)
- SQLite database volume for persistence
- Uses `deploy.sh` for automated deployment

### 11.2 Production Considerations

| Consideration | Current State | Recommendation |
|---|---|---|
| Database | SQLite (single-writer) | Sufficient for single-node; PostgreSQL for multi-node |
| TLS | Built-in (rustls) | Set `TLS_CERT_PATH`/`TLS_KEY_PATH`; or use reverse proxy |
| DTLS | Built-in (openssl PSK) | Enable for CoAP device encryption |
| Logging | `env_logger` (structured) | Consider `tracing` for async span support |
| Monitoring | None | Add Prometheus metrics endpoint |
| Backup | Manual | Automate SQLite backup (VACUUM INTO) |
| Rate limiting | Multi-layer | PIN (exponential backoff), discovery (per-IP), actuator (per-device) |

---

## 12. Test Suites & Coverage

### 12.1 Overview

| Application | Framework | Test Files | Test Count | Coverage |
|---|---|---|---|---|
| **Rust Backend** | `cargo test` | 4 integration + 2 inline | ~134 tests | Flow engine, auth, conditions, flow CRUD, CoAP |
| **SvelteKit Frontend** | Vitest + jsdom | 7 files | ~96 tests | Stores, WebSocket events, models, enums |
| **Android App** | — | 0 files | 0 tests | **None** |
| **CI/CD** | — | — | — | **No pipeline configured** |

**Total: ~230 tests** (excluding emulator tests)

### 12.2 Rust Backend Tests

#### Integration Tests (`homesoil/tests/`)

**`flow_engine_tests.rs`** (82 tests) — Most comprehensive:
- FlowGraph serialization/deserialization roundtrips
- Topological sort (linear, diamond, fan-out, cycle detection)
- Node evaluation for all 5 node types
- NodeValue type conversions (Number ↔ Bool ↔ None)
- Full end-to-end pipeline scenarios (9 cases)
- Type coercion edge cases

**`condition_parser_tests.rs`** (22 tests):
- All comparison operators (==, !=, <, >, <=, >=)
- Type support (int, float, string, boolean)
- Variable substitution
- Array membership (in, not in)

**`flow_methods_tests.rs`** (18 tests):
- CRUD operations with in-memory SQLite
- Graph JSON preservation
- Enable/disable toggle
- Lifecycle testing (insert → update → toggle → delete)

**`auth_tests.rs`** (11 tests):
- PIN generation (8-digit validation, randomness)
- Device secret generation (32-char format)
- Session token generation (48-char format)
- Rate limiting (3 max attempts, exponential backoff, reset, clear)

#### Inline Unit Tests

- **`client.rs`** (~17 tests): CoAP URL parsing, HTTP methods, block transfers
- **`observer.rs`** (3 tests): Observe/unobserve lifecycle

### 12.3 Frontend Tests

**Framework**: Vitest 4.1.0 with jsdom environment

**`WebsocketListenEventMap.test.ts`** (34 tests):
- All server→client event handlers
- Store update verification for every event type

**`Websocket.test.ts`** (23 tests):
- Socket.IO emit methods for all operations
- JSON payload structure verification
- Mock socket.io-client

**`store.test.ts`** (14 tests):
- Store reactivity with spread operator
- CRUD operations on dictionary stores
- Token persistence to localStorage

**`Flow.test.ts`** (10 tests):
- Flow model validation
- Graph JSON parsing
- Dangling edge detection

**`persistant.test.ts`** (6 tests):
- localStorage sync with Svelte stores
- Corrupt data handling

**`WebsocketEmitEventEnum.test.ts`** (4 tests) + **`WebsocketListenEventEnum.test.ts`** (5 tests):
- Event constant uniqueness and completeness

### 12.4 Android Tests

**No tests exist.** The project has no JUnit, Espresso, or Compose UI test files.

---

## 13. Test Improvement Plan

### 13.1 What's Well-Tested

- Flow engine evaluation logic (excellent — 82 tests with edge cases)
- Condition parser operators and types
- Flow CRUD database operations
- Auth credential generation and rate limiting
- Frontend store reactivity patterns
- WebSocket event emission and reception
- Event enum completeness

### 13.2 Critical Gaps

#### Backend — Not Tested

| Area | Impact | Priority |
|---|---|---|
| **Script parser** (~1,104 lines, zero tests) | DSL parsing could silently fail | **Critical** |
| **Sensor/actuator handlers** | Core device management untested | **Critical** |
| **Socket.IO event routing** | 30+ events with no integration tests | **High** |
| **CoAP handler dispatch** | Device registration/commands untested | **High** |
| **Database error paths** | Constraint violations, connection failures | **High** |
| **Intermittent actuator state machine** | Timing-dependent logic | **Medium** |
| **Sensor read cleanup task** | Data retention logic | **Medium** |
| **Health check ping loop** | Online/offline transitions | **Medium** |
| **End-to-end flow: sensor read → flow trigger → actuator** | Core automation pipeline | **High** |

#### Frontend — Not Tested

| Area | Impact | Priority |
|---|---|---|
| **Component rendering** (0 component tests) | UI regressions undetectable | **High** |
| **Flow editor interactions** | Visual node programming untested | **High** |
| **Script editor validation** | Line-by-line error reporting | **Medium** |
| **Authentication/login flow** | User-facing auth UX | **Medium** |
| **Error states and loading** | Degraded UX on failure | **Medium** |
| **Sensor detail chart** | Data visualization correctness | **Low** |

#### Android — Not Tested

| Area | Impact | Priority |
|---|---|---|
| **Everything** (0 tests) | No confidence in any behavior | **Critical** |
| **SocketManager event handling** | Core networking layer | **Critical** |
| **ViewModel state management** | Business logic | **Critical** |
| **Repository data flow** | Data layer correctness | **High** |
| **UI compose screens** | Visual regression | **Medium** |

#### Project-Wide

| Gap | Priority |
|---|---|
| **No CI/CD pipeline** — tests never run automatically | **Critical** |
| **No end-to-end tests** — full user flows untested | **High** |
| **No security tests** — auth bypass, injection | **High** |
| **No load/stress tests** — concurrent device handling | **Medium** |

### 13.3 Recommended Test Additions

#### Phase 1 — Critical (Backend)

1. **Script parser tests** — Add tests for every DSL command, control flow construct, error cases, and edge cases (nested loops, undefined variables, invalid syntax). This is the largest untested module.

2. **Sensor/Actuator handler unit tests** — Test `register`, `unregister`, `name_change`, and `state_change` handlers with mocked database and CoAP client.

3. **Socket.IO event integration tests** — Use socketioxide test utilities to verify event routing and store broadcast.

4. **CoAP handler tests** — Test path routing with mock payloads, device secret validation, malformed JSON handling.

#### Phase 2 — High Priority

5. **End-to-end automation pipeline test** — Simulate: sensor reading arrives → enabled flow evaluates → actuator command sent. Use in-memory DB + mock CoAP client.

6. **Frontend component tests** — Add `@testing-library/svelte` tests for `Sensor.svelte`, `Actuator.svelte`, `FlowEditor.svelte`, `ScriptEditor.svelte`.

7. **Android unit tests** — Add JUnit tests for `SocketManager` event parsing, ViewModel state transitions, `HomesoilRepository` data flow.

8. **Error path tests** (backend) — Test behavior on: database connection failure, malformed JSON, missing device, network timeout, duplicate registration.

#### Phase 3 — Hardening

9. **CI/CD pipeline** — GitHub Actions workflow running `cargo test`, `bun run test:unit`, and Android `./gradlew test` on every push/PR.

10. **Security tests** — Auth bypass attempts, rate limit enforcement under load, XSS in flow/script titles.

11. **Android instrumented tests** — Espresso/Compose UI tests for login flow, dashboard rendering, actuator control.

12. **Performance tests** — Flow evaluation with 100+ nodes, 50+ concurrent sensor readings, large sensor_reads queries.

### 13.4 Test Infrastructure Recommendations

| Recommendation | Applies To | Benefit |
|---|---|---|
| Add `@testing-library/svelte` | Frontend | Component-level rendering tests |
| Add `mockall` crate | Backend | Mock traits for DB, CoAP client |
| Add `Hilt` + JUnit | Android | Dependency injection for testable ViewModels |
| Add GitHub Actions CI | All | Automated test execution on every PR |
| Add `testcontainers` | Backend | Isolated DB instances for integration tests |
| Create test fixtures | All | Shared sample data (sensors, flows, scripts) |
| Add code coverage reporting | All | Track coverage trends over time |

---

*Generated on 2026-04-07 — Updated with security hardening (DTLS, TLS, auth hardening, input validation, resource limits, structured logging)*
