-- Register the Raspberry Pi sensor device
-- Run this in your PostgreSQL database

INSERT INTO devices (device_id, device_name, secret_hash, is_active, created_at, last_seen_at, metadata)
VALUES (
    'RPI-SENSOR-001',
    'Raspberry Pi MAX30102 Sensor',
    'placeholder_hash', -- Not used yet, backend uses config.toml secret
    true,
    now(),
    now(),
    '{"location": "test_environment", "sensor_type": "MAX30102"}'::jsonb
)
ON CONFLICT (device_id) DO UPDATE SET
    is_active = true,
    last_seen_at = now();

-- Verify the device was inserted
SELECT device_id, device_name, is_active, created_at 
FROM devices 
WHERE device_id = 'RPI-SENSOR-001';
