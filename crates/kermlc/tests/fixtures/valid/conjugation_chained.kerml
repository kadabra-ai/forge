package ChainedConjugation {
    type A {
        in feature f : A;
        out feature g : A;
    }
    type B ~ A {}
    type C ~ B {}
}
