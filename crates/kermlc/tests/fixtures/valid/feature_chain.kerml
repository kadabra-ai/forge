package Models {
    type Person {
        feature parents : Person [0..*];
    }
    type Family {
        feature members : Person;
    }
}
