# Architecture

- [Overview](#overview)
- [Three-Layer Architecture](#three-layer-architecture)
  - [Layer 1: Memory Layer](#layer-1-memory-layer)
  - [Layer 2: DBMS Layer](#layer-2-dbms-layer)
  - [Layer 3: API Layer](#layer-3-api-layer)
- [Crate Organization](#crate-organization)
  - [ic-dbms-api](#ic-dbms-api)
  - [ic-dbms-canister](#ic-dbms-canister)
  - [ic-dbms-macros](#ic-dbms-macros)
  - [ic-dbms-client](#ic-dbms-client)
- [Data Flow](#data-flow)
  - [Insert Operation](#insert-operation)
  - [Select Operation](#select-operation)
  - [Transaction Flow](#transaction-flow)
- [Extension Points](#extension-points)

---

## Overview

ic-dbms is built as a layered architecture where each layer has specific responsibilities and builds upon the layer below. This design provides:

- **Separation of concerns**: Each layer focuses on one aspect
- **Testability**: Layers can be tested independently
- **Flexibility**: Internal implementations can change without affecting APIs

---

## Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Layer 3: API Layer                       │
│  Canister endpoints, Candid interface, access control        │
│  (DbmsCanister macro, ACL guards, request/response types)    │
├─────────────────────────────────────────────────────────────┤
│                     Layer 2: DBMS Layer                      │
│  Tables, CRUD operations, transactions, foreign keys         │
│  (TableRegistry, TransactionManager, query execution)        │
├─────────────────────────────────────────────────────────────┤
│                    Layer 1: Memory Layer                     │
│  Stable memory management, encoding/decoding, page allocation│
│  (MemoryProvider, MemoryManager, Encode trait)               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  IC Stable      │
                    │    Memory       │
                    └─────────────────┘
```

### Layer 1: Memory Layer

**Responsibilities:**
- Manage stable memory allocation (64 KiB pages)
- Encode/decode data to/from binary format
- Track free space and handle fragmentation
- Provide abstraction for testing (heap vs stable memory)

**Key components:**

| Component | Purpose |
|-----------|---------|
| `MemoryProvider` | Abstract interface for memory access |
| `MemoryManager` | Allocates and manages pages |
| `Encode` trait | Binary serialization for all stored types |
| `PageLedger` | Tracks which pages belong to which table |
| `FreeSegmentsLedger` | Tracks free space for reuse |

**Memory layout:**

```
Page 0: Schema Registry (table → page mapping)
Page 1: ACL (allowed principals)
Page 2+: Table data (Page Ledger, Free Segments, Records)
```

See [Memory Documentation](./memory.md) for detailed technical information.

### Layer 2: DBMS Layer

**Responsibilities:**
- Implement CRUD operations
- Manage transactions with ACID properties
- Enforce foreign key constraints
- Handle sanitization and validation
- Execute queries with filters

**Key components:**

| Component | Purpose |
|-----------|---------|
| `TableRegistry` | Manages records for a single table |
| `TransactionManager` | Handles transaction lifecycle |
| `Transaction` | Overlay for uncommitted changes |
| `QueryExecutor` | Executes queries with filters |
| `ForeignKeyHandler` | Validates and cascades foreign keys |

**Transaction model:**

```
┌──────────────────────────────────────────┐
│           Active Transactions             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐   │
│  │  Tx 1   │  │  Tx 2   │  │  Tx 3   │   │
│  │ (overlay)│  │(overlay)│  │(overlay)│   │
│  └────┬────┘  └────┬────┘  └────┬────┘   │
│       │            │            │         │
│       └────────────┼────────────┘         │
│                    │                      │
│                    ▼                      │
│         ┌─────────────────┐              │
│         │  Committed Data │              │
│         │   (in memory)   │              │
│         └─────────────────┘              │
└──────────────────────────────────────────┘
```

Transactions use an overlay pattern:
- Changes are written to an overlay (in-memory)
- Reading checks overlay first, then committed data
- Commit merges overlay to committed data
- Rollback discards the overlay

### Layer 3: API Layer

**Responsibilities:**
- Expose Candid interface
- Handle request/response encoding
- Enforce access control (ACL)
- Route requests to DBMS layer
- Generate table-specific endpoints

**Key components:**

| Component | Purpose |
|-----------|---------|
| `DbmsCanister` macro | Generates canister API from schema |
| ACL guard | Checks caller authorization |
| Request types | `InsertRequest`, `UpdateRequest`, `Query` |
| Response types | `Record`, error handling |

**Generated API structure:**

```rust
// For each table "users":
insert_users(UserInsertRequest, Option<TxId>) -> Result<()>
select_users(Query, Option<TxId>) -> Result<Vec<UserRecord>>
update_users(UserUpdateRequest, Option<TxId>) -> Result<u64>
delete_users(DeleteBehavior, Option<Filter>, Option<TxId>) -> Result<u64>

// Global operations:
begin_transaction() -> TxId
commit(TxId) -> Result<()>
rollback(TxId) -> Result<()>
acl_add_principal(Principal) -> Result<()>
acl_remove_principal(Principal) -> Result<()>
acl_allowed_principals() -> Vec<Principal>
```

---

## Crate Organization

```
ic-dbms/
├── ic-dbms-api/        # Shared types and traits
├── ic-dbms-canister/   # Core DBMS implementation
├── ic-dbms-macros/     # Procedural macros
└── ic-dbms-client/     # Client libraries
```

### ic-dbms-api

**Purpose:** Shared types used across all crates

**Contents:**
- Data types (`Uint32`, `Text`, `DateTime`, etc.)
- `Value` enum for runtime values
- Filter and Query types
- Sanitizer and Validator traits
- Error types
- `Table` trait (marker for table types)

**Dependencies:** Minimal (candid, serde)

### ic-dbms-canister

**Purpose:** Core database engine

**Contents:**
- Memory layer implementation
- DBMS layer implementation
- Transaction management
- Built-in sanitizers and validators
- `DbmsCanister` derive macro re-export

**Dependencies:** ic-dbms-api, ic-dbms-macros, ic-cdk

### ic-dbms-macros

**Purpose:** Procedural macros for code generation

**Macros:**
- `#[derive(Encode)]` - Binary serialization
- `#[derive(Table)]` - Table schema and related types
- `#[derive(DbmsCanister)]` - Complete canister API

**Dependencies:** syn, quote, proc-macro2

### ic-dbms-client

**Purpose:** Client libraries for canister interaction

**Implementations:**
- `IcDbmsCanisterClient` - Inter-canister calls
- `IcDbmsAgentClient` - External via ic-agent (feature-gated)
- `IcDbmsPocketIcClient` - Testing with PocketIC (feature-gated)

**Dependencies:** ic-dbms-api, ic-cdk (optional ic-agent, pocket-ic)

---

## Data Flow

### Insert Operation

```
1. Client calls insert_users(request, tx_id)
              │
2. ACL guard checks caller authorization
              │
3. API layer deserializes request
              │
4. DBMS layer:
   a. Apply sanitizers to values
   b. Apply validators to values
   c. Check primary key uniqueness
   d. Validate foreign key references
   e. If tx_id: write to transaction overlay
      Else: write directly
              │
5. Memory layer:
   a. Encode record to bytes
   b. Find space (free segment or new page)
   c. Write to stable memory
              │
6. Return Result<()>
```

### Select Operation

```
1. Client calls select_users(query, tx_id)
              │
2. ACL guard checks caller authorization
              │
3. API layer deserializes query
              │
4. DBMS layer:
   a. Parse filters
   b. Determine pages to scan
   c. For each page:
      - Read records from memory
      - If tx_id: merge with overlay
      - Apply filters
      - Apply ordering
   d. Apply limit/offset
   e. Select requested columns
   f. Handle eager loading
              │
5. Memory layer:
   a. Read pages
   b. Decode records
              │
6. Return Result<Vec<Record>>
```

### Transaction Flow

```
begin_transaction():
  1. Generate transaction ID
  2. Create empty overlay
  3. Record owner (caller principal)
  4. Return transaction ID

Operation with tx_id:
  1. Verify caller owns transaction
  2. Read from: overlay first, then committed
  3. Write to: overlay only

commit(tx_id):
  1. Verify caller owns transaction
  2. For each change in overlay:
     - Write to committed data (stable memory)
  3. Delete overlay
  4. Transaction ID becomes invalid

rollback(tx_id):
  1. Verify caller owns transaction
  2. Delete overlay (discard all changes)
  3. Transaction ID becomes invalid
```

---

## Extension Points

ic-dbms provides several extension points for customization:

### Custom Sanitizers

Implement the `Sanitize` trait:

```rust
pub trait Sanitize {
    fn sanitize(&self, value: Value) -> IcDbmsResult<Value>;
}
```

### Custom Validators

Implement the `Validate` trait:

```rust
pub trait Validate {
    fn validate(&self, value: &Value) -> IcDbmsResult<()>;
}
```

### Custom Data Types

While not directly extensible, you can use `Json` for custom structures:

```rust
pub metadata: Json,  // Store any structure
```

### Memory Provider

For testing, implement `MemoryProvider`:

```rust
pub trait MemoryProvider {
    const PAGE_SIZE: u64;
    fn size(&self) -> u64;
    fn pages(&self) -> u64;
    fn grow(&mut self, new_pages: u64) -> MemoryResult<u64>;
    fn read(&self, offset: u64, buf: &mut [u8]) -> MemoryResult<()>;
    fn write(&mut self, offset: u64, buf: &[u8]) -> MemoryResult<()>;
}
```

ic-dbms provides:
- `IcMemoryProvider` - Uses IC stable memory (production)
- `HeapMemoryProvider` - Uses heap memory (testing)
