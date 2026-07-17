# Exchange Key Rotation Procedure

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** Operator procedure for rotating the `EXCHANGE_SECRET_KEY` (master encryption key for the `exchange_keys` SQLite table) and re-encrypting credentials without losing access to live exchange connections.

---

## 1. Background

The platform encrypts exchange credentials (api_key, api_secret, passphrase) at rest using AES-256-GCM with a master key derived from the `EXCHANGE_SECRET_KEY` environment variable. The encryption contract is in [06-02 §3.8](../integration-and-api/06-02-database-schema-spec.md). The master key is **never persisted**; it lives only in process memory for the duration of the daemon.

If `EXCHANGE_SECRET_KEY` is lost or rotated, all existing `exchange_keys` rows become unreadable. This document describes how to rotate without losing access.

---

## 2. Pre-rotation checklist

1. **Inventory existing keys.** `sqlite3 telemetry.db "SELECT key_id, exchange, created_at, last_rotated_at FROM exchange_keys;"` — record every `key_id`.
2. **Confirm operator UI access** — keys must be re-entered via `POST /api/keys` if rotation requires re-encryption from scratch.
3. **Schedule a maintenance window.** Rotation requires a daemon restart (Ops Phase 1+ will support hot-rotation; v6.0 requires restart).
4. **Backup `telemetry.db`.** `cp telemetry.db telemetry.db.backup-pre-rotation-$(date +%Y%m%d)`.

---

## 3. Rotation procedure

### 3.1 Generate a new master key

```bash
NEW_KEY=$(openssl rand -hex 32)   # 256 bits, hex-encoded
echo "$NEW_KEY" > /tmp/new_exchange_secret_key
chmod 600 /tmp/new_exchange_secret_key
```

### 3.2 Decrypt-and-re-encrypt script (v6.0: manual via API)

v6.0 does **not** ship an in-process rotation tool; the operator uses the documented `POST /api/keys` endpoint. The flow:

1. Stop the daemon: `./manage.sh stop`.
2. Start the daemon with the **old** key: `EXCHANGE_SECRET_KEY=$OLD_KEY ./execution-daemon`.
3. For each existing `key_id`, read the exchange credentials out-of-band (e.g. from the exchange's UI) and re-insert via `POST /api/keys` with the same `exchange`. The new row replaces the old (Ops Phase 2 endpoint will support UPDATE; v6.0 requires DELETE + INSERT).
4. Verify: `sqlite3 telemetry.db "SELECT COUNT(*) FROM exchange_keys;"` — should match the pre-rotation count.
5. Stop the daemon.
6. Start the daemon with the **new** key: `EXCHANGE_SECRET_KEY=$NEW_KEY ./execution-daemon`.
7. Smoke test: confirm a trade tick arrives within 60 s.

### 3.3 Cleanup

```bash
unset NEW_KEY
rm -f /tmp/new_exchange_secret_key
```

---

## 4. Emergency rotation (suspected compromise)

If `EXCHANGE_SECRET_KEY` is suspected to have been exposed:

1. **Immediate:** stop the daemon.
2. **Rotate exchange-side credentials first** (regenerate api_key/api_secret on the exchange's UI).
3. Re-enter the new exchange credentials with the **new** master key (skip §3.2 step 1-2; start directly with the new key on an empty `exchange_keys` table).
4. **Do not reuse** the compromised master key.
5. Audit `telemetry.db` for unexpected `exchange_keys` activity during the exposure window.

---

## 5. Future work (open; target: next minor)

- `POST /api/keys/rotate` — in-process re-encryption under a new master key without a daemon restart.
- Hot key rotation via SIGHUP — daemon re-reads `EXCHANGE_SECRET_KEY` on signal.
- Encrypted-backup export — `GET /api/keys/backup` returns an encrypted blob keyed by a passphrase, suitable for off-machine storage.

These are tracked under `AUDIT-V6-077` (open) in `docs/CHANGELOG.md`.

---

## 6. Cross-References

- Encryption contract: [06-02 §3.8](../integration-and-api/06-02-database-schema-spec.md)
- `/api/keys` endpoint: [06-01 §2.10](../integration-and-api/06-01-api-gateway-contract.md)
- User manual: [08-01](../operations-and-compliance/08-01-user-manual.md)
- Operator identity model: [06-01 §1](../integration-and-api/06-01-api-gateway-contract.md)