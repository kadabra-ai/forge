package Directions {
    type Sensor {
        in feature reading : Sensor;
        out feature signal : Sensor;
        inout feature control : Sensor;
        feature data : Sensor;
    }
}
