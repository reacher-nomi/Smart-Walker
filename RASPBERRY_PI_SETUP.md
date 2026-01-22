# Raspberry Pi Sensor Setup Guide

## 🎯 Get Live Data from MAX30102 Sensor

This guide will help you set up the Raspberry Pi to send **live heart rate, SpO2, and temperature** data to your laptop dashboard.

## 📋 Prerequisites

- Raspberry Pi (any model with I2C support)
- MAX30102 sensor module
- Your laptop IP: `192.168.0.253`
- Backend running on laptop at port `8080`

## 🔧 Hardware Setup

### Connect MAX30102 to Raspberry Pi

| MAX30102 Pin | Raspberry Pi Pin | Purpose |
|--------------|------------------|---------|
| VIN          | Pin 1 (3.3V)     | Power   |
| GND          | Pin 6 (GND)      | Ground  |
| SDA          | Pin 3 (GPIO 2)   | I2C Data|
| SCL          | Pin 5 (GPIO 3)   | I2C Clock|

## 💻 Raspberry Pi Software Setup

### 1. Enable I2C

```bash
sudo raspi-config
# Navigate to: Interface Options → I2C → Enable
# Reboot when prompted
```

### 2. Install Dependencies

```bash
# Update system
sudo apt update
sudo apt upgrade -y

# Install required packages
sudo apt install -y python3-pip python3-smbus i2c-tools

# Install Python libraries
pip3 install smbus requests numpy
```

### 3. Verify I2C Connection

```bash
# Check if sensor is detected (should show address 0x57)
sudo i2cdetect -y 1
```

Expected output:
```
     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
00:          -- -- -- -- -- -- -- -- -- -- -- -- -- 
10: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- 
20: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- 
30: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- 
40: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- 
50: -- -- -- -- -- -- -- 57 -- -- -- -- -- -- -- -- 
60: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- 
70: -- -- -- -- -- -- -- --
```

### 4. Transfer Files to Raspberry Pi

From your laptop, copy the sensor files:

```bash
# On your laptop (PowerShell or Command Prompt)
scp -r "d:\D disk\DIT HI\3 sem\Innovation and Complexity Management\Project\sensor_reader.py" pi@<RPI_IP>:~/medhealth/
```

Or use FileZilla/WinSCP to transfer the `sensor_reader.py` folder.

### 5. Test Backend Connection

On Raspberry Pi:

```bash
# Test if backend is reachable
curl http://192.168.0.253:8080/health
```

Expected response: `{"status":"ok"}`

If this fails, check:
- Both devices are on the same WiFi network
- Windows Firewall allows port 8080 (see below)
- Backend is running on your laptop

## 🚀 Running the Sensor

### Start the Sensor Script

```bash
cd ~/medhealth/sensor_reader.py
python3 send_to_backend.py
```

You should see output like:
```
🏥 MedHealth Sensor Client
==================================================

Testing backend connection...
✅ Backend connection OK

Starting sensor...
⏸️  No finger detected, waiting...
```

### Place Your Finger on the Sensor

- Place your finger **gently** on the sensor
- Don't press too hard
- Wait 5-10 seconds for stable readings

You should see:
```
✅ Sent: HR=75, SpO2=98, Temp=36.5°C
✅ Sent: HR=76, SpO2=97, Temp=36.6°C
```

### Run as Background Service (Optional)

To run automatically on boot:

```bash
# Create systemd service
sudo nano /etc/systemd/system/medhealth-sensor.service
```

Add:
```ini
[Unit]
Description=MedHealth Sensor Monitor
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/medhealth/sensor_reader.py
ExecStart=/usr/bin/python3 /home/pi/medhealth/sensor_reader.py/send_to_backend.py
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable medhealth-sensor
sudo systemctl start medhealth-sensor

# Check status
sudo systemctl status medhealth-sensor

# View logs
sudo journalctl -u medhealth-sensor -f
```

## 🖥️ Windows Laptop Setup

### Allow Backend Through Firewall

**Option 1: PowerShell (Run as Administrator)**
```powershell
New-NetFirewallRule -DisplayName "MedHealth Backend" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow
```

**Option 2: Windows Firewall GUI**
1. Open Windows Defender Firewall
2. Click "Advanced settings"
3. Click "Inbound Rules" → "New Rule"
4. Select "Port" → Next
5. Enter port `8080` → Next
6. Select "Allow the connection" → Next
7. Check all profiles → Next
8. Name it "MedHealth Backend" → Finish

### Verify Backend is Running

```powershell
# Check if backend is listening
netstat -an | Select-String "8080"

# Should show: TCP  0.0.0.0:8080  LISTENING
```

## 📊 View Live Data

Open the dashboard in your browser:
```
file:///d:/D%20disk/DIT%20HI/3%20sem/Innovation%20and%20Complexity%20Management/Project/dashboard.html
```

You should see:
- ✅ **Connected** status (green)
- Live **Heart Rate** updating every 2-3 seconds
- Live **SpO2** (blood oxygen) readings
- Live **Temperature** readings

## 🐛 Troubleshooting

### Sensor Not Detected
```bash
# Check I2C is enabled
sudo raspi-config

# Check sensor is connected
sudo i2cdetect -y 1
```

### Cannot Connect to Backend
```bash
# Ping laptop from Raspberry Pi
ping 192.168.0.253

# Check if port 8080 is open
nc -zv 192.168.0.253 8080
```

### 401 Unauthorized Errors
- Device is already registered in database ✅
- Secret matches: `CHANGE_ME_DEVICE_SECRET` ✅
- If you changed the secret, update both:
  - `sensor_reader.py/send_to_backend.py` line 18
  - `website/backend/config.toml` line 25

### Dashboard Shows Old Data
- Hard refresh: `Ctrl+Shift+R` (Windows) or `Cmd+Shift+R` (Mac)
- Check browser console (F12) for SSE connection errors

### Temperature Always Same Value
The `read_temperature()` function uses simulated values. To get real temperature:
- Add a DS18B20 temperature sensor
- Or use the MAX30102's internal temperature sensor (requires additional implementation)

## 📝 Configuration

### Change Backend URL
Edit `sensor_reader.py/send_to_backend.py`:
```python
BACKEND_URL = "http://YOUR_LAPTOP_IP:8080"
```

### Change Reading Interval
Edit `sensor_reader.py/send_to_backend.py`:
```python
READING_INTERVAL = 2  # seconds (current: 2 seconds)
```

## ✅ System Status Checklist

- [ ] I2C enabled on Raspberry Pi
- [ ] MAX30102 sensor detected (`sudo i2cdetect -y 1` shows 0x57)
- [ ] Dependencies installed (`smbus`, `requests`, `numpy`)
- [ ] Backend reachable from Raspberry Pi (`curl http://192.168.0.253:8080/health`)
- [ ] Windows Firewall allows port 8080
- [ ] Sensor script running (`python3 send_to_backend.py`)
- [ ] Dashboard shows "Connected" status
- [ ] Live data updating on dashboard

## 🎉 Success!

When everything works, you'll see:
- Raspberry Pi console showing `✅ Sent: HR=XX, SpO2=XX, Temp=XX.X°C`
- Dashboard updating in real-time with live readings
- Green "Connected" indicator on dashboard

Enjoy your real-time health monitoring system! 💚
