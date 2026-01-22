
from max30102 import MAX30102
import hrcalc
import threading
import time
import numpy as np

class HeartRateMonitor(object):
    """
    A class that encapsulates the max30102 device into a thread
    """

    LOOP_TIME = 0.01

    def __init__(self, print_raw=False, print_result=False):
        self.bpm = 0
        self.spo2 = 0
        self.temperature = 0.0
        self.finger_detected = False
        if print_raw is True:
            print('IR, Red')
        self.print_raw = print_raw
        self.print_result = print_result

    def run_sensor(self):
        sensor = MAX30102()
        ir_data = []
        red_data = []
        bpms = []
        bpm_avg_list = []
        last_print = time.time()
        last_temp_read = time.time()

        # run until told to stop
        while not self._thread.stopped:
            # check if any data is available
            num_bytes = sensor.get_data_present()
            if num_bytes > 0:
                # grab all the data and stash it into arrays
                while num_bytes > 0:
                    red, ir = sensor.read_fifo()
                    num_bytes -= 1
                    ir_data.append(ir)
                    red_data.append(red)
                    if self.print_raw:
                        print("{0}, {1}".format(ir, red))

                while len(ir_data) > 100:
                    ir_data.pop(0)
                    red_data.pop(0)

                if len(ir_data) == 100:
                    bpm, valid_bpm, spo2, valid_spo2 = hrcalc.calc_hr_and_spo2(ir_data, red_data)
                    
                    # Check for finger presence with better signal quality detection
                    ir_mean = np.mean(ir_data)
                    red_mean = np.mean(red_data)
                    ir_std = np.std(ir_data)
                    
                    # Finger detected if: high DC value AND signal has variation (not flat)
                    finger_detected = (ir_mean > 50000 and ir_std > 100) or (red_mean > 50000 and np.std(red_data) > 100)
                    self.finger_detected = finger_detected
                    
                    if valid_bpm and finger_detected:
                        # Reject outlier readings (unrealistic heart rates)
                        if 40 <= bpm <= 180:  # Normal human range
                            # Additional check: reject readings that are too different from recent average
                            if len(bpms) == 0 or abs(bpm - np.mean(bpms)) < 30:  # Within 30 bpm of average
                                bpms.append(bpm)
                                while len(bpms) > 8:  # Increased from 4 to 8 for better averaging
                                    bpms.pop(0)
                                self.bpm = np.mean(bpms)
                                bpm_avg_list.append(self.bpm)
                        
                        # Store SpO2 value if valid
                        if valid_spo2 and 85 <= spo2 <= 100:  # Stricter SpO2 range
                            self.spo2 = spo2
                        
                        # Read temperature every 2 seconds
                        if time.time() - last_temp_read >= 2:
                            try:
                                self.temperature = sensor.read_temperature()
                                last_temp_read = time.time()
                            except:
                                pass  # Keep last valid temperature
                        
                        if self.print_result:
                            if time.time() - last_print >= 5 and len(bpm_avg_list) > 0:
                                avg_bpm = sum(bpm_avg_list) / len(bpm_avg_list)
                                print("BPM (5s avg): {0:.1f},  SpO2: {1}, Temp: {2}°C".format(avg_bpm, spo2, self.temperature))
                                bpm_avg_list = []
                                last_print = time.time()
                    elif not finger_detected:
                        self.bpm = 0
                        self.spo2 = 0
                        self.finger_detected = False
                        if self.print_result:
                            print("Finger not detected")

            time.sleep(self.LOOP_TIME)

        sensor.shutdown()

    def start_sensor(self):
        self._thread = threading.Thread(target=self.run_sensor)
        self._thread.stopped = False
        self._thread.start()

    def stop_sensor(self, timeout=2.0):
        self._thread.stopped = True
        self.bpm = 0
        self.spo2 = 0
        self.temperature = 0.0
        self.finger_detected = False
        self._thread.join(timeout)
