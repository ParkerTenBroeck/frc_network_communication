
Byte order is Big Endian



| name | bytes 
 -- | -- 
sequence | 2 
comm_version | 1 
control_code | 1
status | 1
battery | 2
request | 1

# Sqeuence

This should be equal to the sequence number from the last recieved packet from the driver station. 

# comm_version

This should always be 1

# control_code

| name | bits
-- | --
mode | 2
enable | 1
fms_attached | 1
brownout_protection | 1
_ | 1
ds_attached | 1
estop | 1

## mode
---
0 = teleop
1 = test
2 = auton
#### 

The remainder of the values are 1 for true and 0 for false

### status

### battery

### request