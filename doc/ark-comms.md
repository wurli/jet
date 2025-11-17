# Jupyter Comm Messages for Ark Kernel

This document describes the exact Jupyter message format to open each comm type supported by the Ark kernel.

## General Jupyter Message Structure

All messages sent to the kernel follow the Jupyter protocol structure:

```json
{
  "header": {
    "msg_id": "unique-message-id",
    "session": "session-id",
    "username": "username",
    "date": "2025-11-15T00:00:00.000Z",
    "msg_type": "comm_open",
    "version": "5.3"
  },
  "parent_header": {},
  "metadata": {},
  "content": {
    "comm_id": "unique-comm-id",
    "target_name": "target-name",
    "data": {}
  }
}
```

## Frontend-Initiated Comms

These comms are opened by the frontend (you) by sending a `comm_open` message on the **Shell** socket.

### 1. Variables Comm (`positron.variables`)

Shows and manages workspace variables.

**Message:**
```json
{
  "header": {
    "msg_id": "msg-variables-001",
    "session": "your-session-id",
    "username": "username",
    "date": "2025-11-15T00:59:00.000Z",
    "msg_type": "comm_open",
    "version": "5.3"
  },
  "parent_header": {},
  "metadata": {},
  "content": {
    "comm_id": "variables-comm-id",
    "target_name": "positron.variables",
    "data": {}
  }
}
```

**Target Name:** `positron.variables`
**Comm ID:** Any unique string (e.g., UUID)
**Data:** Empty object `{}`

---

### 2. UI Comm (`positron.ui`)

General frontend communication for UI operations.

**Message:**
```json
{
  "header": {
    "msg_id": "msg-ui-001",
    "session": "your-session-id",
    "username": "username",
    "date": "2025-11-15T00:59:00.000Z",
    "msg_type": "comm_open",
    "version": "5.3"
  },
  "parent_header": {},
  "metadata": {},
  "content": {
    "comm_id": "ui-comm-id",
    "target_name": "positron.ui",
    "data": {}
  }
}
```

**Target Name:** `positron.ui`
**Comm ID:** Any unique string
**Data:** Empty object `{}`

---

### 3. Help Comm (`positron.help`)

Displays R documentation and help content.

**Message:**
```json
{
  "header": {
    "msg_id": "msg-help-001",
    "session": "your-session-id",
    "username": "username",
    "date": "2025-11-15T00:59:00.000Z",
    "msg_type": "comm_open",
    "version": "5.3"
  },
  "parent_header": {},
  "metadata": {},
  "content": {
    "comm_id": "help-comm-id",
    "target_name": "positron.help",
    "data": {}
  }
}
```

**Target Name:** `positron.help`
**Comm ID:** Any unique string
**Data:** Empty object `{}`

---

### 4. Ark Test Comm (`ark`)

Custom test comm for raw comm functionality testing.

**Message:**
```json
{
  "header": {
    "msg_id": "msg-ark-001",
    "session": "your-session-id",
    "username": "username",
    "date": "2025-11-15T00:59:00.000Z",
    "msg_type": "comm_open",
    "version": "5.3"
  },
  "parent_header": {},
  "metadata": {},
  "content": {
    "comm_id": "ark-comm-id",
    "target_name": "ark",
    "data": {}
  }
}
```

**Target Name:** `ark` (no prefix)
**Comm ID:** Any unique string
**Data:** Empty object `{}`

---

## Backend-Initiated Comms

These comms are opened by the kernel (backend) and sent to the frontend via IOPub channel. You'll receive them as `comm_open` messages, but you don't send them.

### 5. Data Explorer Comm (`positron.dataExplorer`)

**Target Name:** `positron.dataExplorer`

Opened when viewing data frames/tables. The kernel sends this to the frontend with metadata:

```json
{
  "msg_type": "comm_open",
  "content": {
    "comm_id": "generated-uuid",
    "target_name": "positron.dataExplorer",
    "data": {
      "variable_name": "mydata",
      "table_shape": { "num_rows": 100, "num_columns": 5 },
      ...
    }
  }
}
```

---

### 6. Connections Comm (`positron.connection`)

**Target Name:** `positron.connection`

Opened when creating database connections. The kernel sends this with connection metadata:

```json
{
  "msg_type": "comm_open",
  "content": {
    "comm_id": "generated-uuid",
    "target_name": "positron.connection",
    "data": {
      "name": "My Database",
      "language_id": "r",
      "host": "localhost",
      "type": "PostgreSQL",
      ...
    }
  }
}
```

---

## Not Handled in Shell (Listed but No Handler)

These are defined in the `Comm` enum but don't have active handlers in `shell.rs`:

- **`positron.lsp`** - Language Server Protocol (handled separately via LSP server)
- **`positron.dap`** - Debug Adapter Protocol (handled separately via DAP server)
- **`positron.plot`** - Dynamic plots (may be handled differently)
- **`positron.dataViewer`** - Listed in enum but not in shell handler

---

## Notes

1. **Positron Prefix:** Target names starting with `"positron."` are validated. The kernel strips the prefix and matches against known comm types. Unknown positron comms are rejected.

2. **Custom Comms:** Target names without `"positron."` prefix (like `"ark"`) are passed through as `Comm::Other`.

3. **Comm IDs:** Should be unique UUIDs or similar unique identifiers.

4. **Message Flow:**
   - Send `comm_open` on **Shell** socket
   - Kernel responds with `status: busy` on **IOPub**
   - If successful, comm is established
   - If failed, kernel sends `comm_close` on **IOPub**
   - Kernel responds with `status: idle` on **IOPub**

5. **Protocol Files:** Auto-generated protocol definitions are in:
   - `/crates/amalthea/src/comm/data_explorer_comm.rs`
   - `/crates/amalthea/src/comm/help_comm.rs`
   - `/crates/amalthea/src/comm/plot_comm.rs`
   - `/crates/amalthea/src/comm/ui_comm.rs`
   - `/crates/amalthea/src/comm/variables_comm.rs`
   - `/crates/amalthea/src/comm/connections_comm.rs`

## Example with Python (jupyter_client)

```python
import uuid
from jupyter_client import KernelManager

km = KernelManager(kernel_name='ark')
km.start_kernel()
kc = km.client()

# Open a variables comm
comm_id = str(uuid.uuid4())
msg = kc.session.msg('comm_open', {
    'comm_id': comm_id,
    'target_name': 'positron.variables',
    'data': {}
})
kc.shell_channel.send(msg)

# Listen for response
while True:
    msg = kc.iopub_channel.get_msg(timeout=5)
    print(msg)
```
