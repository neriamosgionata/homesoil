-- Add DTLS support tracking to devices
ALTER TABLE sensors ADD COLUMN dtls_supported BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE actuators ADD COLUMN dtls_supported BOOLEAN NOT NULL DEFAULT 0;

-- Clear existing session tokens (they will be stored hashed going forward)
DELETE FROM session_tokens;
