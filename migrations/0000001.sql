CREATE TYPE admin_status_enum AS ENUM ('active', 'disabled');
CREATE TYPE admin_api_key_status_enum AS ENUM ('active', 'revoked');

CREATE TYPE business_status_enum AS ENUM ('active', 'suspended');

CREATE TYPE api_key_status_enum AS ENUM ('active', 'expiring', 'revoked');

CREATE TYPE account_status_enum AS ENUM ('active', 'frozen');

CREATE TYPE transaction_type_enum AS ENUM ('credit', 'debit', 'transfer');
CREATE TYPE transaction_status_enum AS ENUM ('pending', 'succeeded', 'failed');

CREATE TYPE webhook_status_enum AS ENUM ('active', 'disabled');
CREATE TYPE webhook_event_status_enum AS ENUM ('pending', 'delivered', 'failed');

CREATE TABLE admins (
                        id BIGSERIAL PRIMARY KEY,
                        username TEXT NOT NULL UNIQUE,
                        status admin_status_enum NOT NULL DEFAULT 'active',
                        created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);


CREATE TABLE admin_api_keys (
                                id BIGSERIAL PRIMARY KEY,
                                admin_id BIGINT NOT NULL,
                                key_hash TEXT NOT NULL,
                                status admin_api_key_status_enum NOT NULL DEFAULT 'active',
                                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                                last_used_at TIMESTAMPTZ,

                                CONSTRAINT fk_admin_api_keys_admin
                                    FOREIGN KEY (admin_id)
                                        REFERENCES admins(id)
                                        ON DELETE CASCADE
);


CREATE TABLE businesses (
                            id BIGSERIAL PRIMARY KEY,
                            name TEXT NOT NULL,
                            status business_status_enum NOT NULL DEFAULT 'active',
                            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE api_keys (
                          id BIGSERIAL PRIMARY KEY,
                          business_id BIGINT NOT NULL,
                          key_hash TEXT NOT NULL,
                          status api_key_status_enum NOT NULL DEFAULT 'active',
                          expires_at TIMESTAMPTZ,
                          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                          last_used_at TIMESTAMPTZ,

                          CONSTRAINT fk_api_keys_business
                              FOREIGN KEY (business_id)
                                  REFERENCES businesses(id)
                                  ON DELETE CASCADE
);

CREATE TABLE business_accounts (
                                   id BIGSERIAL PRIMARY KEY,
                                   business_id BIGINT NOT NULL,
                                   name TEXT NOT NULL,
                                   balance NUMERIC(18,2) NOT NULL DEFAULT 0,
                                   currency CHAR(3) NOT NULL,
                                   status account_status_enum NOT NULL DEFAULT 'active',
                                   created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

                                   CONSTRAINT fk_business_accounts_business
                                       FOREIGN KEY (business_id)
                                           REFERENCES businesses(id)
                                           ON DELETE CASCADE
);

CREATE TABLE transactions (
                              id BIGSERIAL PRIMARY KEY,
                              business_id BIGINT NOT NULL,
                              from_account_id BIGINT,
                              to_account_id BIGINT,
                              type transaction_type_enum NOT NULL,
                              amount NUMERIC(18,2) NOT NULL,
                              status transaction_status_enum NOT NULL DEFAULT 'pending',
                              reference_id TEXT,
                              idempotency_key TEXT NOT NULL,
                              created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

                              CONSTRAINT fk_transactions_business
                                  FOREIGN KEY (business_id)
                                      REFERENCES businesses(id)
                                      ON DELETE CASCADE,

                              CONSTRAINT fk_transactions_from_account
                                  FOREIGN KEY (from_account_id)
                                      REFERENCES business_accounts(id)
                                      ON DELETE SET NULL,

                              CONSTRAINT fk_transactions_to_account
                                  FOREIGN KEY (to_account_id)
                                      REFERENCES business_accounts(id)
                                      ON DELETE SET NULL,

                              CONSTRAINT uq_transactions_idempotency
                                  UNIQUE (business_id, idempotency_key)
);

CREATE TABLE webhooks (
                          id BIGSERIAL PRIMARY KEY,
                          business_id BIGINT NOT NULL,
                          url TEXT NOT NULL,
                          secret TEXT NOT NULL,
                          status webhook_status_enum NOT NULL DEFAULT 'active',
                          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

                          CONSTRAINT fk_webhooks_business
                              FOREIGN KEY (business_id)
                                  REFERENCES businesses(id)
                                  ON DELETE CASCADE
);

CREATE TABLE webhook_events (
                                id BIGSERIAL PRIMARY KEY,
                                webhook_id BIGINT NOT NULL,
                                event_type TEXT NOT NULL,
                                payload JSONB NOT NULL,
                                attempt_count INT NOT NULL DEFAULT 0,
                                next_retry_at TIMESTAMPTZ,
                                status webhook_event_status_enum NOT NULL DEFAULT 'pending',
                                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

                                CONSTRAINT fk_webhook_events_webhook
                                    FOREIGN KEY (webhook_id)
                                        REFERENCES webhooks(id)
                                        ON DELETE CASCADE
);


CREATE INDEX idx_admin_api_keys_admin_id ON admin_api_keys(admin_id);
CREATE INDEX idx_api_keys_business_id ON api_keys(business_id);
CREATE INDEX idx_business_accounts_business_id ON business_accounts(business_id);

CREATE INDEX idx_transactions_business_id ON transactions(business_id);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);

CREATE INDEX idx_webhooks_business_id ON webhooks(business_id);
CREATE INDEX idx_webhook_events_webhook_id ON webhook_events(webhook_id);
CREATE INDEX idx_webhook_events_status ON webhook_events(status);
