# Transactions

- [Transactions](#transactions)
  - [Overview](#overview)
  - [Transaction Lifecycle](#transaction-lifecycle)
    - [Begin Transaction](#begin-transaction)
    - [Perform Operations](#perform-operations)
    - [Commit](#commit)
    - [Rollback](#rollback)
  - [ACID Properties](#acid-properties)
    - [Atomicity](#atomicity)
    - [Consistency](#consistency)
    - [Isolation](#isolation)
    - [Durability](#durability)
  - [Transaction Ownership](#transaction-ownership)
  - [Error Handling](#error-handling)
    - [Handling Failures](#handling-failures)
    - [Transaction Errors](#transaction-errors)
  - [Best Practices](#best-practices)
    - [1. Keep transactions short](#1-keep-transactions-short)
    - [2. Always handle rollback](#2-always-handle-rollback)
    - [3. Use transactions for related operations](#3-use-transactions-for-related-operations)
    - [4. Don't mix transactional and non-transactional operations](#4-dont-mix-transactional-and-non-transactional-operations)
  - [Examples](#examples)
    - [Bank Transfer](#bank-transfer)
    - [Order Processing](#order-processing)

---

## Overview

ic-dbms supports ACID transactions, allowing you to group multiple database operations into a single atomic unit. Either all operations succeed and are committed together, or none of them take effect.

**Key features:**

- **Atomicity**: All operations in a transaction succeed or fail together
- **Consistency**: Data integrity constraints are maintained
- **Isolation**: Transactions are isolated from each other
- **Durability**: Committed changes persist across canister upgrades

---

## Transaction Lifecycle

### Begin Transaction

Start a new transaction using `begin_transaction()`:

```rust
use ic_dbms_client::{IcDbmsCanisterClient, Client as _};

let client = IcDbmsCanisterClient::new(canister_id);

// Begin a new transaction
let tx_id: u64 = client.begin_transaction().await?;
println!("Started transaction: {}", tx_id);
```

The returned transaction ID is used for all subsequent operations within this transaction.

### Perform Operations

Pass the transaction ID to CRUD operations:

```rust
// Insert within transaction
client
    .insert::<User>(User::table_name(), user, Some(tx_id))
    .await??;

// Update within transaction
client
    .update::<User>(User::table_name(), update, Some(tx_id))
    .await??;

// Delete within transaction
client
    .delete::<User>(User::table_name(), DeleteBehavior::Restrict, Some(filter), Some(tx_id))
    .await??;

// Select within transaction (sees uncommitted changes)
let users = client
    .select::<User>(User::table_name(), query, Some(tx_id))
    .await??;
```

> **Note:** Operations within a transaction are visible to subsequent operations in the same transaction, but not to other callers until committed.

### Commit

Commit the transaction to make all changes permanent:

```rust
// Commit the transaction
client.commit(tx_id).await??;
println!("Transaction committed successfully");
```

After commit:

- All changes become visible to other callers
- The transaction ID becomes invalid
- Changes persist across canister upgrades

### Rollback

Rollback the transaction to discard all changes:

```rust
// Rollback the transaction
client.rollback(tx_id).await??;
println!("Transaction rolled back");
```

After rollback:

- All changes within the transaction are discarded
- The transaction ID becomes invalid
- The database state is as if the transaction never happened

---

## ACID Properties

### Atomicity

All operations in a transaction are treated as a single unit. If any operation fails, the entire transaction can be rolled back:

```rust
let tx_id = client.begin_transaction().await?;

// First operation succeeds
client.insert::<User>(User::table_name(), user1, Some(tx_id)).await??;

// Second operation fails (e.g., primary key conflict)
let result = client.insert::<User>(User::table_name(), user2_duplicate, Some(tx_id)).await?;

if result.is_err() {
    // Rollback everything - user1 is also discarded
    client.rollback(tx_id).await??;
}
```

### Consistency

Transactions maintain data integrity:

- Primary key uniqueness is enforced
- Foreign key constraints are checked
- Validators run on all data
- Sanitizers are applied

```rust
let tx_id = client.begin_transaction().await?;

// This will fail if referenced user doesn't exist
let post = PostInsertRequest {
    id: 1.into(),
    title: "My Post".into(),
    author_id: 999.into(),  // Non-existent user
};

let result = client.insert::<Post>(Post::table_name(), post, Some(tx_id)).await?;
// Returns Err(BrokenForeignKeyReference)
```

### Isolation

Changes made within a transaction are not visible to other callers until committed:

```rust
// Caller A starts a transaction
let tx_a = client_a.begin_transaction().await?;
client_a.insert::<User>(User::table_name(), new_user, Some(tx_a)).await??;

// Caller B queries - does NOT see the new user
let users = client_b.select::<User>(User::table_name(), query, None).await??;
assert!(!users.iter().any(|u| u.id == new_user.id));

// Caller A commits
client_a.commit(tx_a).await??;

// Now Caller B can see the user
let users = client_b.select::<User>(User::table_name(), query, None).await??;
assert!(users.iter().any(|u| u.id == new_user.id));
```

### Durability

Committed transactions persist across canister upgrades. ic-dbms uses stable memory to ensure data survives upgrades.

---

## Transaction Ownership

Transactions are owned by the principal that created them. Only the owner can:

- Perform operations within the transaction
- Commit the transaction
- Rollback the transaction

```rust
// Principal A creates transaction
let tx_id = client_a.begin_transaction().await?;

// Principal A can use it
client_a.insert::<User>(User::table_name(), user, Some(tx_id)).await??;  // OK

// Principal B cannot use it
let result = client_b.insert::<User>(User::table_name(), user, Some(tx_id)).await?;
// Returns Err(TransactionNotFound) or similar error

// Only Principal A can commit
client_a.commit(tx_id).await??;  // OK
```

---

## Error Handling

### Handling Failures

When an operation fails within a transaction, you should typically rollback:

```rust
let tx_id = client.begin_transaction().await?;

async fn process_order(client: &impl Client, tx_id: u64) -> Result<(), IcDbmsError> {
    // Multiple operations that should succeed together
    client.insert::<Order>(Order::table_name(), order, Some(tx_id)).await??;
    client.update::<Inventory>(Inventory::table_name(), update, Some(tx_id)).await??;
    client.insert::<OrderItem>(OrderItem::table_name(), item, Some(tx_id)).await??;
    Ok(())
}

match process_order(&client, tx_id).await {
    Ok(()) => {
        client.commit(tx_id).await??;
        println!("Order processed successfully");
    }
    Err(e) => {
        client.rollback(tx_id).await??;
        println!("Order failed, rolled back: {:?}", e);
    }
}
```

### Transaction Errors

| Error | Cause |
|-------|-------|
| `TransactionNotFound` | Invalid transaction ID or transaction already completed |
| `TransactionNotOwned` | Caller doesn't own the transaction |

```rust
use ic_dbms_api::prelude::{IcDbmsError, TransactionError};

let result = client.commit(invalid_tx_id).await?;
match result {
    Ok(()) => println!("Committed"),
    Err(IcDbmsError::Transaction(TransactionError::NotFound)) => {
        println!("Transaction not found or already completed");
    }
    Err(e) => println!("Other error: {:?}", e),
}
```

---

## Best Practices

### 1. Keep transactions short

Long-running transactions hold resources and block other operations:

```rust
// GOOD: Prepare data outside transaction
let users_to_insert = prepare_users();

let tx_id = client.begin_transaction().await?;
for user in users_to_insert {
    client.insert::<User>(User::table_name(), user, Some(tx_id)).await??;
}
client.commit(tx_id).await??;

// BAD: Doing expensive work inside transaction
let tx_id = client.begin_transaction().await?;
for raw_data in large_dataset {
    let user = expensive_parsing(raw_data);  // Don't do this in transaction
    client.insert::<User>(User::table_name(), user, Some(tx_id)).await??;
}
client.commit(tx_id).await??;
```

### 2. Always handle rollback

Ensure transactions are either committed or rolled back:

```rust
let tx_id = client.begin_transaction().await?;

let result = async {
    client.insert::<User>(User::table_name(), user1, Some(tx_id)).await??;
    client.insert::<User>(User::table_name(), user2, Some(tx_id)).await??;
    Ok::<(), IcDbmsError>(())
}.await;

match result {
    Ok(()) => client.commit(tx_id).await??,
    Err(_) => client.rollback(tx_id).await??,
}
```

### 3. Use transactions for related operations

Group operations that should succeed or fail together:

```rust
// GOOD: Related operations in transaction
let tx_id = client.begin_transaction().await?;
client.insert::<Order>(Order::table_name(), order, Some(tx_id)).await??;
client.insert::<Payment>(Payment::table_name(), payment, Some(tx_id)).await??;
client.update::<Inventory>(Inventory::table_name(), inv_update, Some(tx_id)).await??;
client.commit(tx_id).await??;

// BAD: Unrelated operations in transaction (unnecessary)
let tx_id = client.begin_transaction().await?;
client.insert::<UserPreferences>(prefs_table, prefs, Some(tx_id)).await??;
client.insert::<AuditLog>(log_table, log, Some(tx_id)).await??;  // Unrelated
client.commit(tx_id).await??;
```

### 4. Don't mix transactional and non-transactional operations

```rust
let tx_id = client.begin_transaction().await?;

// GOOD: All operations use the transaction
client.insert::<Order>(Order::table_name(), order, Some(tx_id)).await??;
client.insert::<OrderItem>(OrderItem::table_name(), item, Some(tx_id)).await??;

// BAD: Mixing transaction and non-transaction
client.insert::<Order>(Order::table_name(), order, Some(tx_id)).await??;
client.insert::<AuditLog>(AuditLog::table_name(), log, None).await??;  // Not in transaction!
```

---

## Examples

### Bank Transfer

Transfer money between accounts atomically:

```rust
async fn transfer(
    client: &impl Client,
    from_account: u32,
    to_account: u32,
    amount: Decimal,
) -> Result<(), IcDbmsError> {
    let tx_id = client.begin_transaction().await?;

    // Deduct from source account
    let deduct = AccountUpdateRequest::builder()
        .decrease_balance(amount)
        .filter(Filter::eq("id", Value::Uint32(from_account.into())))
        .build();
    client.update::<Account>(Account::table_name(), deduct, Some(tx_id)).await??;

    // Add to destination account
    let add = AccountUpdateRequest::builder()
        .increase_balance(amount)
        .filter(Filter::eq("id", Value::Uint32(to_account.into())))
        .build();
    client.update::<Account>(Account::table_name(), add, Some(tx_id)).await??;

    // Record the transfer
    let transfer_record = TransferInsertRequest {
        id: Uuid::new_v4().into(),
        from_account: from_account.into(),
        to_account: to_account.into(),
        amount,
        timestamp: DateTime::now(),
    };
    client.insert::<Transfer>(Transfer::table_name(), transfer_record, Some(tx_id)).await??;

    // Commit atomically
    client.commit(tx_id).await??;
    Ok(())
}
```

### Order Processing

Process an order with inventory update:

```rust
async fn process_order(
    client: &impl Client,
    order: OrderInsertRequest,
    items: Vec<OrderItemInsertRequest>,
) -> Result<u32, Box<dyn std::error::Error>> {
    let tx_id = client.begin_transaction().await?;

    // Insert the order
    client.insert::<Order>(Order::table_name(), order.clone(), Some(tx_id)).await??;

    // Insert order items and update inventory
    for item in items {
        // Insert order item
        client.insert::<OrderItem>(OrderItem::table_name(), item.clone(), Some(tx_id)).await??;

        // Decrease inventory
        let inv_update = InventoryUpdateRequest::builder()
            .decrease_quantity(item.quantity)
            .filter(Filter::eq("product_id", Value::Uint32(item.product_id)))
            .build();

        let updated = client.update::<Inventory>(
            Inventory::table_name(),
            inv_update,
            Some(tx_id)
        ).await??;

        if updated == 0 {
            // Product not in inventory, rollback
            client.rollback(tx_id).await??;
            return Err("Product not found in inventory".into());
        }
    }

    // All successful, commit
    client.commit(tx_id).await??;
    Ok(order.id.into())
}
```
