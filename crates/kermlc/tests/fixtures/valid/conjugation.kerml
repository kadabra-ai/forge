package Connectors {
    type Source {
        in feature input : Source;
        out feature output : Source;
        inout feature control : Source;
        feature data : Source;
    }
    type Sink ~ Source {}
}
