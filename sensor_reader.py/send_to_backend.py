#!/usr/bin/env python3
"""
Raspberry Pi Client - Sends sensor data to backend via HMAC-authenticated API
"""

import time
import hmac
import hashlib
import base64
import json
import requests
from heartrate_monitor import HeartRateMonitor
from max30102 import MAX30102

# Configuration
BACKEND_URL = "http://192.168.0.253:8080"  # Your laptop IP
DEVICE_ID = "RPI-SENSOR-001"
DEVICE_SECRET = "CHANGE_ME_DEVICE_SECRET"  # Must match backend config.toml
READING_INTERVAL = 2  # seconds

class BackendClient:
    def __init__(self, base_url, device_id, secret):
        self.base_url = base_url.rstrip('/')
        self.device_id = device_id
        self.secret = secret
        self.session = requests.Session()
    
    def generate_hmac_signature(self, timestamp, json_body):
        """Generate HMAC-SHA256 signature for authentication"""
        message = f"{timestamp}.{json_body}"
        signature = hmac.new(
            self.secret.encode('utf-8'),
            message.encode('utf-8'),
            hashlib.sha256
        ).digest()
        return base64.b64encode(signature).decode('utf-8')
    
    def send_vitals(self, heart_rate, spo2, temperature):
        """Send vitals to backend"""
        timestamp = int(time.time())
        
        payload = {
            "heartRate": heart_rate,
            "spo2": spo2,
            "temperature": temperature,
            "timestamp": timestamp
        }
        
        json_body = json.dumps(payload)
        signature = self.generate_hmac_signature(timestamp, json_body)
        
        headers = {
            "Content-Type": "application/json",
            "X-Device-Id": self.device_id,
            "X-Timestamp": str(timestamp),
            "X-Signature": signature
        }
        
        try:
            response = self.session.post(
                f"{self.base_url}/api/device/vitals",
                headers=headers,
                data=json_body,
                timeout=10
            )
            
            if response.status_code == 200:
                print(f"✅ Sent: HR={heart_rate}, SpO2={spo2}, Temp={temperature:.1f}°C")
                return True
            else:
                print(f"❌ Error {response.status_code}: {response.text}")
                return False
        
        except requests.exceptions.RequestException as e:
            print(f"❌ Network error: {e}")
            return False
    
    def test_connection(self):
        """Test backend connectivity"""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            if response.status_code == 200:
                print("✅ Backend connection OK")
                return True
            else:
                print(f"⚠️  Backend returned {response.status_code}")
                return False
        except requests.exceptions.RequestException as e:
            print(f"❌ Cannot connect to backend: {e}")
            return False


def read_temperature(monitor):
    """Read temperature from MAX30102 sensor"""
    # Temperature is read from the sensor and stored in monitor.temperature
    return monitor.temperature if monitor.temperature > 0 else 0.0


def validate_vitals(heart_rate, spo2, temperature):
    """Validate vital signs are within reasonable ranges"""
    # Allow zeros (means no finger detected - clear display)
    if heart_rate == 0 and spo2 == 0 and temperature == 0:
        return True
    
    # Must have valid heart rate (finger detected)
    if heart_rate == 0:
        return False
    
    # Strict validation - realistic human ranges
    hr_valid = 40 <= heart_rate <= 200
    spo2_valid = 85 <= spo2 <= 100  # SpO2 below 85% is dangerous
    temp_valid = 25 <= temperature <= 45  # Sensor die temperature range
    
    return hr_valid and spo2_valid and temp_valid


def main():
    print("🏥 MedHealth Sensor Client")
    print("=" * 50)
    
    # Initialize backend client
    client = BackendClient(BACKEND_URL, DEVICE_ID, DEVICE_SECRET)
    
    # Test connection
    print("\nTesting backend connection...")
    if not client.test_connection():
        print("\n⚠️  Warning: Backend not accessible. Will retry on each reading.")
    
    # Initialize heart rate monitor
    print("\nInitializing MAX30102 sensor...")
    monitor = HeartRateMonitor(print_raw=False, print_result=True)
    monitor.start_sensor()
    
    print("\n🚀 Monitoring started. Press Ctrl+C to stop.\n")
    
    last_finger_state = False
    
    try:
        while True:
            # Check if finger is detected
            finger_detected = monitor.finger_detected and monitor.bpm > 0
            
            # If finger was detected but now removed, send zeros to clear dashboard
            if last_finger_state and not finger_detected:
                print("⏸️  Finger removed - clearing display...")
                client.send_vitals(0, 0, 0)  # Clear dashboard
                last_finger_state = False
            
            # Only send real data if finger is detected
            if not finger_detected:
                print("⏸️  No finger detected - place finger on sensor...")
                time.sleep(READING_INTERVAL)
                continue
            
            # Get current readings from sensor
            heart_rate = int(monitor.bpm)
            spo2 = int(monitor.spo2) if monitor.spo2 > 0 else 0
            temperature = read_temperature(monitor)
            
            # Require all valid readings (no zeros)
            if heart_rate == 0 or spo2 == 0 or temperature == 0:
                print("⏸️  Waiting for stable readings...")
                time.sleep(READING_INTERVAL)
                continue
            
            last_finger_state = True
            
            # Validate data before sending
            if validate_vitals(heart_rate, spo2, temperature):
                client.send_vitals(heart_rate, spo2, temperature)
            else:
                print(f"⚠️  Invalid readings: HR={heart_rate}, SpO2={spo2}, Temp={temperature:.1f}°C")
            
            time.sleep(READING_INTERVAL)
    
    except KeyboardInterrupt:
        print("\n\n🛑 Stopping sensor monitoring...")
    
    finally:
        monitor.stop_sensor()
        print("✅ Sensor stopped. Goodbye!")


if __name__ == "__main__":
    main()
