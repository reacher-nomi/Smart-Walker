-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Users table with enhanced security
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK (role IN ('admin', 'viewer', 'clinician')),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ,
    failed_login_attempts INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role) WHERE is_active = true;

-- Devices table for IoT authentication
CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    device_id TEXT NOT NULL UNIQUE,
    device_name TEXT NOT NULL,
    secret_hash TEXT NOT NULL, -- Store HMAC secret hashed
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_devices_device_id ON devices(device_id) WHERE is_active = true;

-- Sensor readings table (raw data)
CREATE TABLE IF NOT EXISTS sensor_readings (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    heart_rate INTEGER CHECK (heart_rate >= 0 AND heart_rate <= 300),
    spo2 INTEGER CHECK (spo2 >= 0 AND spo2 <= 100),
    temperature REAL CHECK (temperature >= 25.0 AND temperature <= 45.0),
    reading_timestamp TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    quality_score REAL CHECK (quality_score >= 0.0 AND quality_score <= 1.0),
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_sensor_readings_device_id ON sensor_readings(device_id);
CREATE INDEX idx_sensor_readings_timestamp ON sensor_readings(reading_timestamp DESC);
CREATE INDEX idx_sensor_readings_received_at ON sensor_readings(received_at DESC);

-- FHIR Observations table (FHIR-compliant storage)
CREATE TABLE IF NOT EXISTS fhir_observations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sensor_reading_id BIGINT NOT NULL REFERENCES sensor_readings(id) ON DELETE CASCADE,
    resource JSONB NOT NULL, -- Full FHIR Observation resource
    resource_type TEXT NOT NULL DEFAULT 'Observation',
    subject_reference TEXT, -- Patient reference
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_fhir_observations_reading_id ON fhir_observations(sensor_reading_id);
CREATE INDEX idx_fhir_observations_subject ON fhir_observations(subject_reference);
CREATE INDEX idx_fhir_resource_gin ON fhir_observations USING gin(resource);

-- ML Analysis results table
CREATE TABLE IF NOT EXISTS ml_analysis (
    id BIGSERIAL PRIMARY KEY,
    sensor_reading_id BIGINT NOT NULL REFERENCES sensor_readings(id) ON DELETE CASCADE,
    anomaly_detected BOOLEAN NOT NULL DEFAULT false,
    anomaly_score REAL CHECK (anomaly_score >= 0.0 AND anomaly_score <= 1.0),
    classification TEXT, -- 'normal', 'warning', 'critical', 'artifact'
    alert_level TEXT CHECK (alert_level IN ('none', 'low', 'medium', 'high', 'critical')),
    analysis_details JSONB DEFAULT '{}'::jsonb,
    analyzed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ml_analysis_reading_id ON ml_analysis(sensor_reading_id);
CREATE INDEX idx_ml_analysis_anomaly ON ml_analysis(anomaly_detected, alert_level);

-- Audit log table (HIPAA compliance)
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL, -- 'login', 'data_access', 'data_modification', 'export'
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id TEXT,
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type, created_at DESC);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);

-- JWT revocation table (for logout/security)
CREATE TABLE IF NOT EXISTS revoked_tokens (
    jti UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_revoked_tokens_expires ON revoked_tokens(expires_at);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for users table
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to cleanup expired revoked tokens (run periodically)
CREATE OR REPLACE FUNCTION cleanup_expired_tokens()
RETURNS void AS $$
BEGIN
    DELETE FROM revoked_tokens WHERE expires_at < now();
END;
$$ LANGUAGE plpgsql;
