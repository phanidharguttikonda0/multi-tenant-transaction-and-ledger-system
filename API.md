# API Documentation

Base URL: `http://localhost:4545`

## Authentication

The API uses **Bearer Token** authentication.
- **Header**: `Authorization: Bearer <YOUR_API_KEY>`

There are two types of keys:
1.  **Admin API Keys**: For managing businesses (Admin routes).
2.  **Business API Keys**: For managing accounts and transactions (Business routes).

## 1. System & Bootstrap

### Health Check
**GET** `/health`
- **Response**: `200 OK` "OK"

### Bootstrap Admin
Initializes the first admin account if none exists.
**GET** `/_internal/bootstrap/admin`
- **Response**:
```json
{
  "status": "success",
  "data": 1  // Admin ID
}
```

### Demo Webhook Listener
A utility endpoint to test webhook delivery. Use this URL when registering a webhook to see if your local server receives events.
**POST** `/demo-webhook-listening`
- **URL**: `http://127.0.0.1:4545/demo-webhook-listening`
- **Note**: This endpoint simply logs the received payload to the server console.

---

## 2. Admin APIs
**Auth Required**: Admin API Key

### Create Business
**POST** `/admin/businesses`
- **Body**:
```json
{
  "name": "Acme Corp"
}
```
- **Response**:
```json
{
  "status": "success",
  "data": 12 // Business ID
}
```

### Get All Businesses
**GET** `/admin/businesses`
- **Response**:
```json
{
  "status": "success",
  "data": [
    {
      "id": 12,
      "name": "Acme Corp",
      "status": "active",
      "created_at": "..."
    }
  ]
}
```

### Generate Business API Key
**POST** `/admin/businesses/api-keys`
- **Body**: `12` (Raw Integer: The Business ID)
- **Response**:
```json
{
  "status": "success",
  "data": "raw_key_..." // Save this key immediately!
}
```

### Rotate Business API Key
**POST** `/admin/api-keys/{key_id}/{business_id}/rotate`
- **Body**: `[1, 12]` (JSON Array: `[key_id, business_id]`)
- **Response**:
```json
{
  "status": "success",
  "data": "new_raw_key_..."
}
```

### Revoke Business API Key
**DELETE** `/admin/api-keys/{key_id}`
- **Body**: `1` (Raw Integer: The Key ID)
- **Response**:
```json
{
  "status": "success",
  "data": "revoked"
}
```

### Generate Admin API Key
**POST** `/admin/admin-api-keys`
- **Body**: `1` (Raw Integer: The Admin ID)
- **Response**:
```json
{
  "status": "success",
  "data": "raw_key_..."
}
```

### Revoke Admin API Key
**DELETE** `/admin/admin-api-keys/{key_id}`
- **Body**: `1` (Raw Integer: The Key ID)
- **Response**:
```json
{
  "status": "success",
  "data": "revoked"
}
```


---

## 3. Business Accounts APIs
**Auth Required**: Business API Key

### Get Your Business Details
**GET** `/get-business-account`
- **Response**: Returns your business profile.

### Get All Accounts
**GET** `/accounts`
- **Response**:
```json
{
  "status": "success",
  "data": [
    {
      "id": 101,
      "name": "Main Wallet",
      "currency": "USD",
      "balance": "1000.00",
      "status": "active"
    }
  ]
}
```

### Create Account
**POST** `/accounts`
- **Body**:
```json
{
  "name": "Marketing Budget",
  "currency": "USD"
}
```
- **Response**:
```json
{
  "status": "success",
  "data": 102 // Account ID
}
```

### Get Account Details
**GET** `/accounts/{account_id}`
- **Response**: Returns full account object.

### Get Account Balance
**GET** `/accounts/{account_id}/balance`
- **Response**:
```json
{
  "status": "success",
  "data": "1000.00"
}
```

---

## 4. Transaction APIs
**Auth Required**: Business API Key
**Requirement**: All POST requests MUST include a unique `idempotency_key` field.

### Credit Account (Deposit)
**POST** `/transaction/credit`
- **Body**:
```json
{
  "to_account_id": 101,
  "amount": "100.00",
  "idempotency_key": "uuid-1",
  "reference_id": "deposit-ref-01"
}
```
- **Response**: `{ "data": 5001 }` (Transaction ID)

### Debit Account (Withdraw)
**POST** `/transaction/debit`
- **Body**:
```json
{
  "from_account_id": 101,
  "amount": "50.00",
  "idempotency_key": "uuid-2",
  "reference_id": "withdraw-ref-01"
}
```

### Transfer Funds
**POST** `/transaction/transfer`
- **Body**:
```json
{
  "from_account_id": 101,
  "to_account_id": 102,
  "amount": "25.00",
  "idempotency_key": "uuid-3",
  "reference_id": "internal-transfer"
}
```

### Get All Transactions
**GET** `/transaction`
- **Response**: List of all transactions for your business.

### Get Transaction Details
**GET** `/transaction/{transaction_id}`
- **Response**:
```json
{
  "status": "success",
  "data": {
    "id": 5001,
    "type": "credit",
    "amount": "100.00",
    "status": "succeeded",
    "business_id": 12,
    "from_account_id": null,
    "to_account_id": 101,
    "reference_id": "deposit-ref-01",
    "idempotency_key": "uuid-1",
    "created_at": "..."
  }
}
```

---

## 5. Webhook APIs
**Auth Required**: Business API Key

### Register Webhook
**POST** `/webhooks`
- **Body**:
```json
{
  "url": "https://your-server.com/webhooks"
}
```
> **Tip**: To test if webhooks are working locally, use the demo endpoint: `http://127.0.0.1:4545/demo-webhook-listening`, such
that every notification sent to the following url.

- **Response**: `{ "data": 1 }` (Webhook ID)

### Get Webhooks
**GET** `/webhooks`
- **Response**: List of registered webhooks.

### Update Webhook
**PUT** `/webhooks/{webhook_id}`
- **Body**:
```json
{
  "url": "https://new-url.com/webhooks",
  "status": "active" // or "disabled"
}
```

### Delete Webhook (Disable)
**DELETE** `/webhooks/{webhook_id}`
- **Response**: `{ "data": "webhook disabled" }`

---

## Webhook Events
When a transaction succeeds, your registered URL will receive a `POST` request.

**Payload**:
```json
{
  "event_type": "transaction.succeeded",
  "payload": {
    "event": "transaction.succeeded",
    "data": {
      "transaction_id": 5001,
      "type": "credit",
      "amount": "100.00",
      "to_account_id": 101,
      "business_id": 12,
      "reference_id": "deposit-ref-01"
    }
  }
}
```
