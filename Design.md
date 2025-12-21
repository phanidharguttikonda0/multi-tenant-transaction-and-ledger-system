# System Design Specification

## Overview
This document outlines the architectural design, data models, and operational considerations for the Multi-Tenant Transaction Service. The system is designed to provide a secure, reliable, and scalable platform for businesses to manage accounts, perform transactions (credits, debits, transfers), and receive real-time updates via webhooks.

## Assumptions & Constraint Decisions
1.  **Concurrency model**: We strictly enforce atomicity using database-level locking (`SELECT ... FOR UPDATE`). While this impacts throughput compared to optimistic locking, it guarantees data correctness, which is paramount for financial ledgers.
2.  **Authentication**: Security is handled via long-lived API keys. We assume businesses are responsible for rotating these keys securely.
3.  **Currency**: The current version assumes a single currency or handled externally by the caller. Multi-currency support is a future extension.
4.  **Deployment**: The system is containerized via Docker for portable deployment, adhering to 12-factor app principles.

## Architecture

The system follows a clean, monolithic architecture built with **Rust (Axum framewwork)**, maximizing performance and memory safety.

### Components
1.  **API Layer (Axum)**: Handles HTTP requests, authentication, and input validation.
2.  **Service Layer**: Encapsulates business logic (check balances, lock accounts, execute transfers).
3.  **Persistence Layer (PostgreSQL)**: The source of truth for all data. Uses ACID transactions to ensure integrity.
4.  **Worker Layout (Tokio)**:
    - **Webhook Worker**: Asynchronously processes event delivery to ensure API latency is not affected by external webhook consumers.
    - **Redis Expiry Listener**: (Optional/Bonus) Handles ephemeral keys for rate limiting and potential future features.
5.  **Infrastructure**:
    - **Redis**: Used for rate limiting (Token Bucket/Fixed Window) and ephemeral state.
    - **PostgreSQL**: Relational data storage.

## Data Schema
The database schema is designed in 3rd Normal Form to reduce redundancy.
*(Refer to `public/ER-Diagram.png` in the repository for the visual entity-relationship diagram)*

### Key Tables
- **businesses**: Represents the tenants of the system.
- **business_accounts**: Sub-ledgers for a business (e.g., "Main Wallet", "Marketing Fund").
- **transactions**: Ledger of all money movements.
    - `idempotency_key`: Ensures duplicate requests are handled safely without double-spending.
    - `type`: Enum (`credit`, `debit`, `transfer`).
- **webhooks & webhook_events**: Implements the reliable delivery outbox pattern. Events are stored transactionally with the ledger update, ensuring consistency.

## API Design
The API utilizes REST principles with JSON payloads.
- **Authentication**: `Authorization: Bearer <API_KEY>`
- **Rate Limiting**: 20 requests per minute per IP address (enforced via Redis middleware).
- **Idempotency**: All mutating endpoints (`POST`) require an `idempotency_key` header or body field to guarantee safe retries.

## Webhook Design
We implement an **"At-Least-Once"** delivery guarantee.
1.  **Transactional Enqueue**: When a transaction succeeds, a `webhook_event` is inserted into the DB within the same SQL transaction. This eliminates the "dual write" problem.
2.  **Async Processor**: A background worker polls for `pending` events and pushes them to the registered HTTPS endpoint.
3.  **Retry Policy**: Exponential backoff is applied for failed deliveries. Delivery status is tracked (`pending`, `delivered`, `failed`).

## Operational Considerations
1.  **Observability**: Structured logging is implemented using `tracing`. Logs are emitted to stdout (for collection by Fluentd/Datadog).
2.  **Health Checks**: `/health` endpoint exposes service status for load balancers (e.g., K8s liveness probes).
3.  **Database Migrations**: Managed via `sqlx`, ensuring schema changes are versioned and reproducible.
4.  **Security**:
    - API Keys are hashed (`SHA256`) before storage.
    - Rate limiting prevents basic DDoS and abuse.
    - Input validation prevents SQL injection and generic bad data.

## Trade-offs
1.  **Database as Queue**:
    - *Decision*: We store webhook events in Postgres instead of a dedicated broker like RabbitMQ.
    - *Expertise*: This simplifies the stack (fewer moving parts) and guarantees transactional coupling with the ledger.
    - *Trade-off*: Increases DB load. For "Twitter-scale", we would simply change the worker to read from DB and push to Kafka, but for a transaction ledger, the consistency benefit outweighs the cost.
2.  **Pessimistic Locking**:
    - *Decision*: We lock rows during transfers.
    - *Trade-off*: Reduces concurrent throughput on a *single* account but guarantees zero race conditions (e.g., preventing negative balances).
3.  **Monolith**:
    - *Decision*: Single generic service.
    - *Trade-off*: Simpler deployment and debugging vs independent scaling of 'read' and 'write' paths.

