package InlineConj {
    type Source {
        in feature input : Source;
        out feature output : Source;
        inout feature control : Source;
        feature data : Source;
    }

    type Wrapper {
        feature port : ~Source;
    }
}
