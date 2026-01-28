from max30102 import MAX30102
import hrcalc
import requests
import time
import numpy as np

# Initialize sensor
sensor = MAX30102()

print("Starting sensor readings...")

ir_data = []
red_data = []

while True:
    try:
        num_bytes = sensor.get_data_present()
        
        if num_bytes > 0:
            while num_bytes > 0:
                red, ir = sensor.read_fifo()
                ir_data.append(ir)
                red_data.append(red)
                num_bytes -= 1
                
                # Keep only last 100 samples
                if len(ir_data) > 100:
                    ir_data.pop(0)
                    red_data.pop(0)
                
                # Calculate HR and SpO2 when we have enough data
                if len(ir_data) == 100:
                    bpm, valid_bpm, spo2, valid_spo2 = hrcalc.calc_hr_and_spo2(ir_data, red_data)
                    
                    if valid_bpm and valid_spo2:
                        # Send data to backend
                        data = {
                            'temp': 36.5,  # You can add temperature if available
                            'hr': int(bpm),
                            'time': int(time.time())
                        }
                        
                        try:
                            response = requests.post('http://127.0.0.1:8080/api/data', json=data)
                            print(f"Sent: HR={bpm} bpm, SpO2={spo2}%")
                        except Exception as e:
                            print(f"Error sending data: {e}")
        
        time.sleep(0.1)
        
    except KeyboardInterrupt:
        print("Stopping...")
        sensor.shutdown()
        break