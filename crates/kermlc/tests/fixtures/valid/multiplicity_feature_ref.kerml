package MultRef {
    type Connection {
        feature portCount : Anything [1];
        feature ports : Anything [1..portCount];
    }
}
