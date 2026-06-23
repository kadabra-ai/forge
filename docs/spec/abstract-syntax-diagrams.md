# KerML Abstract Syntax — Mermaid Diagrams

Reference diagrams from KerML 1.0 Beta 2 spec, Chapter 8.3.

## Figure 9. Types (8.3.3.1)

```mermaid
classDiagram
    direction TB

    Namespace <|-- Type

    class Type {
        +isAbstract : Boolean = false
        +isSufficient : Boolean = false
        +inheritedMemberships(excluded : Namespace[0..*], isRecursive : Boolean, includeAll : Boolean) Membership[0..*]
        +directionOf(feature : Feature) FeatureDirectionKind[0..1]
        +removeRedefinedFeatures(memberships : Membership[0..*]) Membership[0..*]
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

    Membership <|-- OwningMembership
    OwningMembership <|-- FeatureMembership

    %% Type -> Membership: ownedIntersectingType
    Type "0..*" --> "0..*" Type : +intersectingType\n{ordered}
    Type --> "0..*" Membership : +inheritedMembership\n{ordered}

    %% Type specialization relationships
    Type "0..*" --> "0..*" Type : +unioningType\n{ordered}
    Type "0..*" --> "0..*" Type : +differencingType\n{ordered}

    %% Feature relationships
    Type --> "0..*" Feature : +feature\n{ordered}
    Type --> "0..*" Feature : +ownedFeature\n{subsets feature, ordered}
    Type --> "0..*" Feature : +directedFeature\n{subsets feature, ordered}
    Type --> "0..*" Feature : +endFeature\n{subsets feature, ordered}

    %% FeatureMembership
    FeatureMembership --> "1" Type : +owningType\n{subsets membershipOwningNamespace}
    FeatureMembership --> "1" Feature : +ownedMemberFeature\n{redefines ownedMemberElement}

    %% Feature -> Type
    Feature --> "0..*" Type : +featuringType\n{ordered}
    Feature --> "0..1" Type : +owningType\n{subsets featuringType,\nsubsets owningNamespace}

    %% Multiplicity
    Feature --> "0..1" Multiplicity : +multiplicity
    Multiplicity --> "0..1" Namespace : namespace
    Multiplicity --|> Feature
    Multiplicity --> "0..*" Membership : +ownedMembership

    %% Specific feature subtypes
    Type --> "0..*" Feature : +input\n{subsets directedFeature, ordered}
    Type --> "0..*" Feature : +output\n{subsets directedFeature, ordered}
```

## Figure 3. Dependencies (8.3.3)

```mermaid
classDiagram
    direction TB

    Element <|-- Relationship
    Relationship <|-- Dependency

    class Element {
    }

    class Relationship {
    }

    class Dependency {
    }

    Dependency "0..*" --> "1..*" Element : +client\n{redefines source, ordered}
    Dependency "0..*" --> "1..*" Element : +supplier\n{redefines target, ordered}

    Element --> "0..*" Dependency : +clientDependency\n{subsets sourceRelationship}
    Element --> "0..*" Dependency : +supplierDependency\n{subsets targetRelationship}
```

## Figure 4. Annotations (8.3.4)

```mermaid
classDiagram
    direction TB

    Element <|-- AnnotatingElement
    AnnotatingElement <|-- Comment
    AnnotatingElement <|-- TextualRepresentation
    Comment <|-- Documentation
    Relationship <|-- Annotation

    class Element {
    }

    class AnnotatingElement {
    }

    class Comment {
        +locale : String[0..1]
        +body : String[1]
    }

    class TextualRepresentation {
        +language : String[1]
        +body : String[1]
    }

    class Documentation {
    }

    class Annotation {
    }

    class Relationship {
    }

    %% Element -> AnnotatingElement
    Element "1" --> "0..1" Element : +/owningAnnotatedElement\n{subsets annotatedElement,\nsubsets owningRelatedElement}
    Element "1" --> "1..*" AnnotatingElement : +/annotatedElement\n{ordered}

    %% Element -> representedElement
    Element "1" --> "1" Element : +/representedElement\n{subsets owner,\nredefines annotatedElement}

    %% Element -> documentedElement
    Element "1" --> "1" Element : +/documentedElement\n{subsets owner,\nredefines annotatedElement}

    %% Annotation relationships
    Annotation "0..*" --> "1" AnnotatingElement : +/annotatingElement\n{redefines source}
    Annotation "0..*" --> "1..*" Element : +/annotatedElement\n{redefines target}
    Element --> "0..*" Annotation : +/ownedAnnotation\n{subsets annotation,\nsubsets ownedRelationship, ordered}

    %% AnnotatingElement self-reference
    AnnotatingElement --> "0..*" AnnotatingElement : +/annotatingElement\n{ordered}

    %% AnnotatingElement -> Annotation
    AnnotatingElement --> "0..1" Annotation : +/owningAnnotatingRelationship\n{subsets annotation,\nsubsets owningRelationship}

    %% TextualRepresentation
    TextualRepresentation --> "0..*" Element : +/textualRepresentation\n{subsets annotatingElement,\nsubsets ownedElement, ordered}

    %% Documentation
    Documentation --> "0..*" Element : +/documentation\n{subsets annotatingElement,\nsubsets ownedElement, ordered}
```

## Figure 5. Namespaces (8.3.5)

```mermaid
classDiagram
    direction TB

    Element <|-- Namespace
    Relationship <|-- Membership
    Membership <|-- OwningMembership

    class Element {
    }

    class Namespace {
        +namesOf(element : Element) String[0..*]
        +visibilityOf(mem : Membership) VisibilityKind
        +visibleMemberships(excluded : Namespace[0..*], isRecursive : Boolean, includeAll : Boolean) Membership[0..*]
        +importedMemberships(excluded : Namespace[0..*]) Membership[0..*]
        +membershipsOfVisibility(visibility : VisibilityKind[0..1], excluded : Namespace[0..*]) Membership[0..*]
        +resolve(qualifiedName : String) Membership[0..1]
        +resolveGlobal(qualifiedName : String) Membership[0..1]
        +resolveLocal(name : String) Membership[0..1]
        +resolveVisible(name : String) Membership[0..1]
        +qualificationOf(qualifiedName : String) String[0..1]
        +unqualifiedNameOf(qualifiedName : String) String
    }

    class Membership {
        +memberElementId : String
        +memberShortName : String[0..1]
        +memberName : String[0..1]
        +visibility : VisibilityKind = public
        +isDistinguishableFrom(other : Membership) Boolean
    }

    class OwningMembership {
        +ownedMemberElementId : String ~~redefines memberElementId~~
        +ownedMemberShortName : String[0..1] ~~redefines memberShortName~~
        +ownedMemberName : String[0..1] ~~redefines memberName~~
        +path() String ~~redefines path~~
    }

    class VisibilityKind {
        <<enumeration>>
        private
        protected
        public
    }

    %% Element -> Membership
    Element "1" --> "1" Membership : +memberElement\n{redefines target}
    Element "1" --> "1" OwningMembership : +/ownedMemberElement\n{subsets ownedRelatedElement,\nredefines memberElement}

    %% Namespace -> Membership
    Namespace "1..*" --> "0..*" Membership : +/membershipNamespace\n{union}
    Namespace "1" --> "0..*" Membership : +/membershipOwningNamespace\n{subsets membershipNamespace,\nsubsets owningRelatedElement,\nredefines source}
    Namespace --> "0..*" Membership : +/ownedMembership\n{subsets membership,\nsubsets ownedRelationship,\nsubsets sourceRelationship, ordered}
    Namespace --> "0..*" Membership : +/importedMembership\n{subsets membership, ordered}

    %% Namespace -> Element
    Namespace --> "0..*" Element : +/member\n{ordered}
    Namespace --> "0..*" Element : +/ownedMember\n{subsets member, ordered}

    %% Namespace -> Namespace
    Namespace --> "0..1" Namespace : +/owningNamespace\n{subsets namespace}
    Namespace --> "0..*" Namespace : +/namespace

    %% Namespace -> ImportingNamespace
    Namespace --> "0..*" Namespace : +/importingNamespace\n{subsets membershipNamespace}

    %% Membership -> Element (target)
    Membership "0..*" --> "0..*" Element : +membership\n{subsets targetRelationship}
```

## Figure 6. Imports (8.3.5)

```mermaid
classDiagram
    direction TB

    Relationship <|-- Import
    Import <|-- MembershipImport
    Import <|-- NamespaceImport

    class Relationship {
    }

    class Import {
        +visibility : VisibilityKind = private
        +isRecursive : Boolean = false
        +isImportAll : Boolean = false
        +importedMemberships(excluded : Namespace[0..*]) Membership[0..*]
    }

    class MembershipImport {
        +importedMemberships(excluded : Namespace[0..*]) Membership[0..*] ~~redefines importedMemberships~~
    }

    class NamespaceImport {
        +importedMemberships(excluded : Namespace[0..*]) Membership[0..*] ~~redefines importedMemberships~~
    }

    class Element {
    }

    class Namespace {
    }

    class Membership {
    }

    %% Import -> Element
    Import --> "1" Element : +/importedElement\n{redefines target}

    %% Import -> Namespace
    Import --> "0..*" Namespace : +/membershipImport
    Import --> "1" Namespace : +/importOwningNamespace\n{subsets owningRelatedElement,\nredefines source}

    %% Import -> ownedImport
    Namespace --> "0..*" Import : +/ownedImport\n{subsets ownedRelationship,\nsubsets sourceRelationship, ordered}

    %% MembershipImport -> Membership
    MembershipImport --> "0..*" Membership : +import\n{redefines targetRelationship}
    MembershipImport --> "1" Membership : +importedMembership\n{redefines target}
    MembershipImport --> "0..*" Membership : +membership\n{subsets targetRelationship}

    %% NamespaceImport -> Namespace
    NamespaceImport --> "0..*" Namespace : +import\n{subsets targetRelationship}
    NamespaceImport --> "1" Namespace : +importedNamespace\n{redefines target}

    %% Element -> memberElement
    Element "1" --> "1" Membership : +memberElement\n{redefines target}
```

## Figure 7. Packages (8.3.5)

```mermaid
classDiagram
    direction TB

    Namespace <|-- Package
    Package <|-- LibraryPackage
    OwningMembership <|-- ElementFilterMembership

    class Package {
        +importedMemberships(excluded : Namespace[0..*]) Membership[0..*] ~~redefines importedMemberships~~
        +includeAsMember(element : Element) Boolean
    }

    class LibraryPackage {
        +isStandard : Boolean = false
        +libraryNamespace() Namespace[0..1] ~~redefines libraryNamespace~~
    }

    class OwningMembership {
    }

    class ElementFilterMembership {
    }

    class Expression {
    }

    %% Package -> LibraryPackage
    Package --> "0..1" Package : +/conditionedPackage\n{subsets owningNamespace}

    %% ElementFilterMembership
    ElementFilterMembership --> "0..1" Package : +/owningFilter\n{subsets owningNamespace}
    ElementFilterMembership --> "0..1" OwningMembership : {subsets owningMembership}
    ElementFilterMembership --> "1" Expression : +/condition\n{redefines ownedMemberElement}

    %% Expression
    Expression --> "0..*" Expression : +/filterCondition\n{subsets ownedMember, ordered}
```

## Figure 8. Definition and Usage — Overview (8.3.6.1)

```mermaid
classDiagram
    direction TB

    Classifier <|-- Definition
    Feature <|-- Usage
    Usage <|-- ReferenceUsage

    class Classifier {
    }

    class Feature {
    }

    class Definition {
        +isVariation : Boolean
    }

    class Usage {
        +/mayTimeVary : Boolean ~~redefines isVariable~~
        +isReference : Boolean
        +isVariation : Boolean
        +namingFeature() Feature[0..1] ~~redefines namingFeature~~
        +referencedFeatureTarget() Feature
    }

    class ReferenceUsage {
        +/isReference : Boolean = true ~~redefines isReference~~
        +namingFeature() Feature[0..1] ~~redefines namingFeature~~
    }

    %% Definition -> Classifier
    Definition --> "0..*" Classifier : +/definition\n{redefines type, ordered}

    %% Definition <-> Usage
    Definition --> "0..*" Usage : +/definedUsage\n{subsets typedFeature}
    Definition --> "0..*" Usage : +/ownedUsage\n{subsets ownedFeature,\nsubsets usage, ordered}
    Definition --> "0..1" Usage : +/owningDefinition\n{subsets featuringDefinition,\nsubsets owningType}

    %% Usage -> Feature
    Usage --> "0..*" Feature : +/usage\n{subsets feature, ordered}
    Usage --> "0..*" Feature : +/directedUsage\n{subsets directedFeature,\nsubsets usage, ordered}

    %% Usage -> Definition
    Usage --> "0..*" Definition : +/featuringDefinition\n{subsets typeWithFeature}

    %% Usage -> owning
    Usage --> "0..1" Usage : +/owningUsage\n{subsets owningType}
    Usage --> "0..*" Usage : +/nestedUsage\n{subsets ownedFeature,\nsubsets usage, ordered}
    Usage --> "0..*" Usage : +/nestedReference\n{subsets nestedUsage, ordered}

    %% Definition -> directed usage
    Definition --> "0..*" Usage : +/definitionWithDirectedUsage\n{subsets featuringDefinition,\nsubsets typeWithDirectedFeature}
    Usage --> "0..*" Usage : +/usageWithDirectedUsage\n{subsets featuringUsage,\nsubsets typeWithDirectedFeature}

    %% ReferenceUsage
    Usage --> "0..*" ReferenceUsage : +/ownedReference\n{subsets ownedUsage, ordered}
    ReferenceUsage --> "0..1" Usage : +/referenceOwningUsage\n{subsets owningUsage}
    Definition --> "0..1" Definition : +/referenceOwningDefinition\n{subsets owningDefinition}
```

## Figure 9. Variant Membership (8.3.6)

```mermaid
classDiagram
    direction TB

    OwningMembership <|-- VariantMembership

    class OwningMembership {
    }

    class VariantMembership {
    }

    class Definition {
    }

    class Usage {
    }

    %% VariantMembership -> Definition
    VariantMembership --> "0..1" Definition : +/owningVariationDefinition\n{subsets membershipOwningNamespace}
    Definition --> "0..1" Definition : +/owningVariationDefinition\n{subsets owningNamespace}

    %% VariantMembership -> Usage
    VariantMembership --> "0..*" Usage : +/variantMembership\n{subsets ownedMembership}
    VariantMembership --> "0..1" Usage : +/owningVariationUsage\n{subsets membershipOwningNamespace}

    %% Definition -> variant
    Definition --> "0..*" Usage : +/variant\n{subsets ownedMember}

    %% Usage -> variant
    Usage --> "0..*" Usage : +/variant\n{subsets ownedMember}
    Usage --> "0..1" Usage : +/owningVariationUsage\n{subsets owningNamespace}

    %% VariantMembership -> ownedVariantUsage
    VariantMembership --> "1" Usage : +/ownedVariantUsage\n{redefines ownedMemberElement}
    VariantMembership --> "0..1" VariantMembership : +/owningVariantMembership\n{subsets owningMembership}
```
