package P {
    type A {
        feature x : A;
    }
    type B {
        feature a : A;
        feature bad chains a.nonexistent;
    }
}
