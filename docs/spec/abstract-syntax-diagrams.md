# KerML Abstract Syntax — Mermaid Diagrams

Reference diagrams from KerML 1.0 Beta 2 spec, Chapter 8.3.
Source: screenshots in `docs/spec/kermlc/`.

## Figure 9. Types (8.3.3.1)

```mermaid
classDiagram
    direction TB

    Namespace <|-- Type

    class Type {
        +isAbstract : Boolean = false
        +isSufficient : Boolean = false
        +visibleMemberships(excluded, isRecursive, includeAll) Membership[0..*]
        +inheritedMemberships(excluded : Namespace[0..*], isRecursive : Boolean, includeAll : Boolean) Membership[0..*]
        +removeRedefinedFeatures(memberships : Membership[0..*]) Membership[0..*]
        +directionOf(feature : Feature, excluded : Type[0..*]) FeatureDirectionKind[0..1]
        +directionOfExcluding(feature : Feature, excluded : Type[0..*]) FeatureDirectionKind[0..1]
        +allSupertypes() Type[0..*]
        +specializes(supertype : Type) Boolean
        +isCompatibleWith(otherType : Type) Boolean
        +multiplicity() Multiplicity[0..1]
    }

    class Membership {
    }

    class OwningMembership {
    }

    class FeatureMembership {
    }

    class Feature {
    }

    class Multiplicity {
    }

    class FeatureDirectionKind {
        <<enumeration>>
        in
        out
        inout
    }

    Membership <|-- OwningMembership
    OwningMembership <|-- FeatureMembership
    Feature --|> Multiplicity

    %% Type -> inherited membership
    Type --> "0..*" Membership : +/inheritedMembership\n{ordered}

    %% FeatureMembership -> owningType / ownedMemberFeature
    FeatureMembership --> "0..1" Type : +/owningType\n{subsets membershipOwningNamespace}
    FeatureMembership --> "1" Feature : +/ownedMemberFeature\n{redefines ownedMemberElement}

    %% Type -> Feature derived properties
    Type --> "0..*" Feature : +/feature\n{ordered}
    Type --> "0..*" Feature : +/ownedFeature\n{subsets feature, ordered}
    Type --> "0..*" Feature : +/directedFeature\n{subsets feature, ordered}
    Type --> "0..*" Feature : +/endFeature\n{subsets feature, ordered}
    Type --> "0..*" Feature : +/input\n{subsets directedFeature, ordered}
    Type --> "0..*" Feature : +/output\n{subsets directedFeature, ordered}

    %% Feature -> Type
    Feature --> "0..*" Type : +/featuringType\n{ordered}
    Feature --> "0..1" Type : +/owningType\n{subsets featuringType,\nsubsets owningNamespace}

    %% Feature -> Multiplicity
    Feature --> "0..1" Multiplicity : +/multiplicity

    %% Multiplicity
    Multiplicity --> "0..1" Namespace : +/namespace
```

## Figure 10. Specialization (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Specialization

    class Specialization {
    }

    class Type {
    }

    %% generalization: Specialization -> Type (general)
    Specialization "0..*" --> "1" Type : +generalization / +general\n{redefines targetRelationship}\n{redefines target}

    %% specialization: Specialization -> Type (specific)
    Specialization "0..*" --> "1" Type : +specialization / +specific\n{subsets sourceRelationship}\n{redefines source}

    %% ownedSpecialization / owningType
    Specialization "0..*" --> "0..1" Type : +/ownedSpecialization / +/owningType\n{subsets ownedRelationship,\nsubsets specialization, ordered}\n{subsets owningRelatedElement,\nsubsets specific}
```

## Figure 11. Conjugation (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Conjugation

    class Conjugation {
    }

    class Type {
    }

    %% conjugation -> originalType
    Conjugation "0..*" --> "1" Type : +conjugation / +originalType\n{subsets targetRelationship}\n{redefines target}

    %% conjugator -> conjugatedType
    Conjugation "0..1" --> "1" Type : +conjugator / +conjugatedType\n{subsets sourceRelationship}\n{redefines source}

    %% ownedConjugator / owningType
    Conjugation "0..1" --> "0..1" Type : +/ownedConjugator / +/owningType\n{subsets conjugator,\nsubsets ownedRelationship}\n{subsets conjugatedType,\nsubsets owningRelatedElement}
```

## Figure 12. Disjoining (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Disjoining

    class Disjoining {
    }

    class Type {
    }

    %% typeDisjoined (source) -> disjoiningTypeDisjoining
    Type "1" --> "0..*" Disjoining : +typeDisjoined / +disjoiningTypeDisjoining\n{redefines source}\n{subsets sourceRelationship}

    %% disjoiningType (target) -> disjoinedTypeDisjoining
    Type "1" --> "0..*" Disjoining : +disjoiningType / +disjoinedTypeDisjoining\n{redefines target}\n{subsets targetRelationship}

    %% owningType / ownedDisjoining
    Type "0..1" --> "0..*" Disjoining : +/owningType / +/ownedDisjoining\n{subsets owningRelatedElement,\nsubsets typeDisjoined}\n{subsets disjoiningTypeDisjoining,\nsubsets ownedRelationship}
```

## Figure 13. Unioning (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Unioning

    class Unioning {
    }

    class Type {
    }

    %% Type self-references
    Type --> "0..*" Type : +/unionedType
    Type --> "0..*" Type : +/unioningType\n{ordered}

    %% typeUnioned / ownedUnioning
    Type "1" --> "0..*" Unioning : +/typeUnioned / +/ownedUnioning\n{subsets owningRelatedElement,\nredefines source}\n{subsets ownedRelationship,\nsubsets sourceRelationship, ordered}

    %% unioningType / unionedUnioning
    Type "1" --> "0..*" Unioning : +unioningType / +unionedUnioning\n{redefines target}\n{subsets targetRelationship}
```

## Figure 14. Intersecting (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Intersecting

    class Intersecting {
    }

    class Type {
    }

    %% Type self-references
    Type --> "0..*" Type : +/intersectedType
    Type --> "0..*" Type : +/intersectingType\n{ordered}

    %% typeIntersected / ownedIntersecting
    Type "1" --> "0..*" Intersecting : +/typeIntersected / +/ownedIntersecting\n{subsets owningRelatedElement,\nredefines source}\n{subsets ownedRelationship,\nsubsets sourceRelationship, ordered}

    %% intersectingType / intersectedIntersecting
    Type "1" --> "0..*" Intersecting : +intersectingType / +intersectedIntersecting\n{redefines target}\n{subsets targetRelationship}
```

## Figure 15. Differencing (8.3.3.1)

```mermaid
classDiagram
    direction LR

    Relationship <|-- Differencing

    class Differencing {
    }

    class Type {
    }

    %% Type self-references
    Type --> "0..*" Type : +/differencedType
    Type --> "0..*" Type : +/differencingType\n{ordered}

    %% typeDifferenced / ownedDifferencing
    Type "1" --> "0..*" Differencing : +/typeDifferenced / +/ownedDifferencing\n{subsets owningRelatedElement,\nredefines source}\n{subsets ownedRelationship,\nsubsets sourceRelationship, ordered}

    %% differencingType / differencedDifferencing
    Type "1" --> "0..*" Differencing : +differencingType / +differencedDifferencing\n{redefines target}\n{subsets targetRelationship}
```

## Figure 17. Features (8.3.3.3)

```mermaid
classDiagram
    direction TB

    Relationship <|-- TypeFeaturing
    Specialization <|-- FeatureTyping
    Type <|-- Feature

    class Feature {
        +isAbstract : Boolean = false
        +isOrdered : Boolean = false
        +isComposite : Boolean = false
        +isEnd : Boolean = false
        +isPortion : Boolean = false
        +isVariable : Boolean = false
        +isConstant : Boolean = false
        +direction : FeatureDirectionKind [0..1]
        +directionFor(type : Type) FeatureDirectionKind[0..1]
        +effectiveName() String[0..1]
        +redefinesFromLibrary(libraryFeatureName : String) Boolean
        +ownedFeatureChain_first(feature_source : Feature) Boolean
        +isCompatibleWith(otherType : Type) Boolean ~~redefines isCompatibleWith~~
        +isCartesianProduct() Boolean
        +ownedChaelFeature() Feature[0..1]
        +allRefinedFeatures() Feature[0..*]
        +isFeatureOfWith(type : Type[0..1]) Boolean
        +isFeaturingType() Type Boolean
    }

    class TypeFeaturing {
    }

    class FeatureTyping {
    }

    class FeatureDirectionKind {
        <<enumeration>>
        in
        out
        inout
    }

    %% TypeFeaturing relationships
    TypeFeaturing "0..*" --> "1" Type : +/typeFeauring\n{subsets targetRelationship}
    TypeFeaturing "0..*" --> "1" Feature : +/featureOfType\n{redefines source}

    %% ownedTypeFeaturing / owningFeature
    TypeFeaturing "0..*" --> "0..1" Feature : +/ownedTypeFeaturing / +/owningFeatureOfType\n{subsets ownerRelationship,\nsubsets typeFeauring, ordered}\n{subsets featureOfType,\nsubsets owningRelatedElement}

    %% FeatureTyping relationships
    FeatureTyping --> "1" Type : +/type\n{redefines general}
    FeatureTyping --> "1" Feature : +/typedFeature\n{redefines specific}

    %% ownedFeatureTyping / owningFeature
    FeatureTyping "0..*" --> "0..1" Feature : +/ownedFeatureTyping / +/owningFeature\n{subsets ownedSpecialization,\nsubsets typing, ordered}\n{subsets typedFeature,\nredefines owningType}

    %% Feature -> Type
    Feature --> "0..*" Type : +/featuringType\n{ordered}
    Feature --> "0..*" Type : +/type\n{ordered}

    %% Feature -> Feature
    Feature --> "0..*" Feature : +/typedFeature\n{subsets generalization}
```

## Figure 18. Subsetting (8.3.3.3)

```mermaid
classDiagram
    direction TB

    Specialization <|-- Subsetting
    Subsetting <|-- Redefinition
    Subsetting <|-- ReferenceSubsetting

    class Subsetting {
    }

    class Redefinition {
    }

    class ReferenceSubsetting {
    }

    class Feature {
    }

    %% Subsetting -> Feature (supersetting / subsettedFeature)
    Subsetting "0..*" --> "1" Feature : +supersetting / +subsettedFeature\n{subsets generalization}\n{redefines general}

    %% Subsetting -> Feature (subsetting / subsettingFeature)
    Subsetting "0..*" --> "1" Feature : +subsetting / +subsettingFeature\n{subsets specialization}\n{redefines specific}

    %% ownedSubsetting / owningFeature
    Subsetting "0..*" --> "0..1" Feature : +/ownedSubsetting / +/owningFeature\n{subsets ownedSpecialization,\nsubsets subsetting}\n{subsets subsettingFeature,\nredefines owningType}

    %% Redefinition -> Feature (redefining / redefinedFeature)
    Redefinition "0..*" --> "1" Feature : +redefining / +redefinedFeature\n{subsets supersetting}\n{redefines subsettedFeature}

    %% Redefinition -> Feature (redefinition / redefiningFeature)
    Redefinition "0..*" --> "1" Feature : +redefinition / +redefiningFeature\n{subsets subsetting}\n{redefines subsettingFeature}

    %% ownedRedefinition / owningFeature
    Redefinition "0..*" --> "0..1" Feature : +/ownedRedefinition / +/owningFeature\n{subsets ownedSubsetting}\n{subsets owningFeature}

    %% ReferenceSubsetting -> Feature (referencing / referencedFeature)
    ReferenceSubsetting "0..*" --> "1" Feature : +referencing / +referencedFeature\n{subsets supersetting}\n{redefines subsettedFeature}

    %% ownedReferenceSubsetting / referencingFeature
    ReferenceSubsetting "0..1" --> "1" Feature : +/ownedReferenceSubsetting / +/referencingFeature\n{subsets ownedSubsetting}\n{redefines owningFeature,\nredefines subsettingFeature}
```

## Figure 19. Feature Chaining (8.3.3.3)

```mermaid
classDiagram
    direction LR

    Relationship <|-- FeatureChaining

    class FeatureChaining {
    }

    class Feature {
    }

    %% Feature self-references
    Feature --> "1" Feature : +/featureTarget\n(0..* -> 1)
    Feature --> "0..*" Feature : +/baseFeature
    Feature --> "0..*" Feature : +/chainedFeature
    Feature --> "0..*" Feature : +/chainingFeature\n{ordered, nonunique}

    %% featureChained / ownedFeatureChaining
    Feature "1" --> "0..*" FeatureChaining : +/featureChained / +/ownedFeatureChaining\n{subsets owningRelatedElement,\nredefines source}\n{subsets ownedRelationship,\nsubsets sourceRelationship, ordered}

    %% chainingFeature / chainedFeatureChaining
    Feature "1" --> "0..*" FeatureChaining : +chainingFeature / +chainedFeatureChaining\n{redefines target}\n{subsets targetRelationship}
```

## Figure 20. Feature Inverting (8.3.3.3)

```mermaid
classDiagram
    direction LR

    Relationship <|-- FeatureInverting

    class FeatureInverting {
    }

    class Feature {
    }

    %% featureInverted / invertingFeatureInverting
    Feature "1" --> "0..*" FeatureInverting : +featureInverted / +invertingFeatureInverting\n{redefines source}\n{subsets sourceRelationship}

    %% invertingFeature / invertedFeatureInverting
    Feature "0..*" --> "0..*" FeatureInverting : +invertingFeature / +invertedFeatureInverting\n{redefines target}\n{subsets targetRelationship}

    %% owningFeature / ownedFeatureInverting
    Feature "0..1" --> "0..*" FeatureInverting : +/owningFeature / +/ownedFeatureInverting\n{subsets featureInverted,\nsubsets owningRelatedElement}\n{subsets invertingFeatureInverting,\nsubsets ownedRelationship}
```

## Figure 21. End Feature Membership (8.3.3.3)

```mermaid
classDiagram
    direction TB

    FeatureMembership <|-- EndFeatureMembership

    class EndFeatureMembership {
    }

    class Feature {
    }

    %% owningEndFeatureMembership / ownedMemberFeature
    EndFeatureMembership "0..1" --> "1" Feature : +/owningEndFeatureMembership / +/ownedMemberFeature\n{subsets owningFeatureMembership}\n{redefines ownedMemberFeature}
```

## Figure 22. Cross Subsetting (8.3.3.3)

```mermaid
classDiagram
    direction LR

    Subsetting <|-- CrossSubsetting

    class CrossSubsetting {
    }

    class Feature {
    }

    %% Feature self-references
    Feature --> "0..1" Feature : +/crossFeature
    Feature --> "0..*" Feature : +/featureCrossing

    %% crossSupersetting / crossedFeature
    CrossSubsetting "0..1" --> "1" Feature : +crossSupersetting / +crossedFeature\n{subsets supersetting}\n{redefines subsettedFeature}

    %% ownedCrossSubsetting / crossingFeature
    CrossSubsetting "0..1" --> "1" Feature : +/ownedCrossSubsetting / +/crossingFeature\n{subsets ownedSubsetting}\n{redefines owningFeature,\nredefines subsettingFeature}
```
