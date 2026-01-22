-- Register the Raspberry Pi sensor device
-- Run this in your PostgreSQL database

INSERT INTO devices (
    device_id, 
    device_name, 
    secret_hash, 
    is_active, 
    created_at, 
    last_seen_at, 
    metadata
)
VALUES (
    'RPI-SENSOR-001',
    'Raspberry Pi Health Sensor',
    crypt('CHANGE_ME_DEVICE_SECRET', gen_salt('bf')), -- Hash the secret using bcrypt
    true,
    now(),
    now(),
    '{"description": "Main health monitoring sensor", "location": "Raspberry Pi", "version": "1.0"}'::jsonb
)
ON CONFLICT (device_id) 
DO UPDATE SET 
    secret_hash = EXCLUDED.secret_hash,
    is_active = EXCLUDED.is_active,
    last_seen_at = now(),
    metadata = EXCLUDED.metadata;

-- Verify the device was registered
SELECT device_id, device_name, is_active, created_at 
FROM devices 
WHERE device_id = 'RPI-SENSOR-001';
