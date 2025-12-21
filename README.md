# Multi-Tenant Transaction Service

A high-performance, concurrent transaction ledger service built with Rust, Axum, Postgres, and Redis.

## Features
- **Account Management**: Create businesses and sub-accounts.
- **Transactions**: Credit, Debit, and Transfer with ACID guarantees.
- **Reliability**: Idempotency keys support.
- **Webhooks**: Reliable event delivery with retries.
- **Security**: API Key authentication and Rate Limiting (20 req/min).

## Prerequisites
- Docker & Docker Compose

## Quick Start
Run the entire stack with a single command:

```bash
docker-compose up --build
```

The service will start on `http://localhost:4545`.

## Manual Setup (Development)
If you prefer running without Docker:
1. Ensure Postgres and Redis are running.
2. Create a `.env` file based on `.env.example`.
3. Run `cargo run`.

## Usage Guide
Follow this sequence to test the flow:

### 1. Bootstrap Admin
Initialize the system.
```bash
curl http://localhost:4545/_internal/bootstrap/admin
# Returns: {"status":"success","data":1}
```

### 2. Generate Admin API Key
```bash
curl -X POST http://localhost:4545/admin/admin-api-keys \
  -H "Content-Type: application/json" \
  -d "1"
# Returns: {"status":"success","data":"<ADMIN_KEY>"}
```
*Save the returned key.*

### 3. Create a Business
```bash
curl -X POST http://localhost:4545/admin/businesses \
  -H "Authorization: Bearer <ADMIN_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Tech Startup"}'
# Returns: {"status":"success","data":<BUSINESS_ID>}
```

### 4. Generate Business API Key
```bash
curl -X POST http://localhost:4545/admin/businesses/api-keys \
  -H "Authorization: Bearer <ADMIN_KEY>" \
  -H "Content-Type: application/json" \
  -d "<BUSINESS_ID>"
# Returns: {"status":"success","data":"<BUSINESS_KEY>"}
```
*Use this key for all subsequent requests.*

### 5. Create Accounts
```bash
# Create Wallet 1
curl -X POST http://localhost:4545/accounts \
  -H "Authorization: Bearer <BUSINESS_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"name": "Main Wallet", "currency": "USD"}'

# Create Wallet 2
curl -X POST http://localhost:4545/accounts \
  -H "Authorization: Bearer <BUSINESS_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"name": "Expense Wallet", "currency": "USD"}'
```

### 6. Perform Transaction (Credit)
```bash
curl -X POST http://localhost:4545/transaction/credit \
  -H "Authorization: Bearer <BUSINESS_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "to_account_id": <ACCOUNT_ID_1>,
    "amount": "1000.00",
    "idempotency_key": "unique-key-001",
    "reference_id": "funding-round-1"
  }'
```

### 7. Register Webhook
```bash
curl -X POST http://localhost:4545/webhooks \
  -H "Authorization: Bearer <BUSINESS_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"url": "http://127.0.0.1:4545/demo-webhook-listening"}' # Use demo endpoint for testing
```

See [API.md](API.md) for full documentation.
See [DESIGN.md](DESIGN.md) for architectural details.
