use crate::error::OxidowlError;
use crate::ontology::axioms::{
    Axiom, ClassAssertionAxiom, DeclarationAxiom, DifferentIndividualsAxiom,
    DisjointClassesAxiom, DisjointObjectPropertiesAxiom, DisjointUnionAxiom, Entity,
    EquivalentClassesAxiom, EquivalentObjectPropertiesAxiom, FunctionalObjectPropertyAxiom,
    InverseFunctionalObjectPropertyAxiom, InverseObjectPropertiesAxiom,
    IrreflexiveObjectPropertyAxiom, ObjectPropertyAssertionAxiom, ObjectPropertyDomainAxiom,
    ObjectPropertyRangeAxiom, ReflexiveObjectPropertyAxiom, SameIndividualAxiom,
    SubClassOfAxiom, SubObjectPropertyOfAxiom, SymmetricObjectPropertyAxiom,
    TransitiveObjectPropertyAxiom, AsymmetricObjectPropertyAxiom, HasKeyAxiom,
    AnnotationPropertyDomainAxiom, AnnotationPropertyRangeAxiom,
};
use crate::ontology::{
    Class, ClassExpression, IRI, Individual, ObjectProperty, ObjectPropertyExpression, Ontology,
};
use std::collections::HashMap;

/// Configuration for Manchester Syntax Parser
#[derive(Debug, Clone)]
pub struct ManchesterParserConfig {
    pub strict_mode: bool,
    pub allow_anonymous_individuals: bool,
    pub custom_prefixes: HashMap<String, String>,
}

impl Default for ManchesterParserConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            allow_anonymous_individuals: false,
            custom_prefixes: HashMap::new(),
        }
    }
}

/// Manchester Syntax Parser for OWL 2
/// Implements parsing according to the Manchester OWL Syntax specification
#[derive(Debug, Clone)]
pub struct ManchesterParser {
    #[allow(dead_code)]
    config: ManchesterParserConfig,
    prefixes: HashMap<String, String>,
    current_position: usize,
    input: String,
}

impl ManchesterParser {
    #[must_use]
    pub fn new(config: ManchesterParserConfig) -> Self {
        let mut prefixes = HashMap::new();

        // Add standard prefixes
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        // Add custom prefixes from config
        prefixes.extend(config.custom_prefixes.clone());

        Self {
            config,
            prefixes,
            current_position: 0,
            input: String::new(),
        }
    }

    /// Parse Manchester Syntax from string
    pub fn parse_string(&mut self, content: &str) -> Result<Ontology, OxidowlError> {
        // Use strict validation for Manchester syntax
        let validator = super::validation::SyntaxValidator::new();
        validator
            .validate_manchester(content)
            .map_err(|e| OxidowlError::ParseError(format!("Manchester validation failed: {e}")))?;

        self.input = content.to_string();
        self.current_position = 0;

        let mut ontology = Ontology::new();
        let mut next_axiom_id: u64 = 1;

        // First pass: parse prefix declarations
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("Prefix:") {
                self.parse_prefix_declaration(line)?;
            }
        }

        // Store prefixes on the ontology
        for (prefix_name, iri) in &self.prefixes {
            ontology.add_prefix(prefix_name.clone(), IRI::new(iri));
        }

        // Parse ontology-level annotations (Annotations: ... after Ontology: header)
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Skip prefix lines (already handled)
            if line.starts_with("Prefix:") || line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                i += 1;
                continue;
            }

            // Handle Ontology: header
            if line.starts_with("Ontology:") {
                i += 1;
                continue;
            }

            // Handle Import: header
            if line.starts_with("Import:") {
                let rest = line.strip_prefix("Import:").unwrap().trim();
                let iri = self.resolve_iri(rest)
                    .map_err(|e| OxidowlError::ParseError(format!("Invalid import IRI: {e}")))?;
                ontology.imports.push(crate::ontology::ImportsDeclaration {
                    imported_ontology_iri: iri,
                });
                i += 1;
                continue;
            }

            // Use the "Annotations:" keyword at ontology level
            if line.starts_with("Annotations:") {
                i += 1;
                continue;
            }

            // Frame headers
            if line.starts_with("Class:") {
                let name = line.strip_prefix("Class:").unwrap().trim();
                let axioms = self
                    .parse_class_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else if line.starts_with("ObjectProperty:") {
                let name = line.strip_prefix("ObjectProperty:").unwrap().trim();
                let axioms = self
                    .parse_object_property_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else if line.starts_with("DataProperty:") {
                let name = line.strip_prefix("DataProperty:").unwrap().trim();
                let axioms = self
                    .parse_data_property_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else if line.starts_with("Individual:") {
                let name = line.strip_prefix("Individual:").unwrap().trim();
                let axioms = self
                    .parse_individual_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else if line.starts_with("Datatype:") {
                let name = line.strip_prefix("Datatype:").unwrap().trim();
                let axioms = self
                    .parse_datatype_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else if line.starts_with("AnnotationProperty:") {
                let name = line.strip_prefix("AnnotationProperty:").unwrap().trim();
                let axioms = self
                    .parse_annotation_property_frame(name, &lines, &mut i, &mut next_axiom_id)?;
                for axiom in axioms {
                    ontology.add_axiom(axiom);
                }
            } else {
                i += 1;
            }
        }

        Ok(ontology)
    }

    /// Parse prefix declaration
    fn parse_prefix_declaration(&mut self, line: &str) -> Result<(), OxidowlError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let prefix_name = parts[1].trim_end_matches(':');
            let iri = parts[2].trim_start_matches('<').trim_end_matches('>');
            self.prefixes
                .insert(prefix_name.to_string(), iri.to_string());
        }
        Ok(())
    }

    /// Parse a class frame: "Class: Name\n  SubClassOf: ...\n  EquivalentTo: ..."
    fn parse_class_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let class_iri = self.resolve_iri(name)?;
        let mut axioms: Vec<Axiom> = Vec::new();

        // Declaration axiom
        axioms.push(Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::Class(class_iri.clone()),
        }));

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            // Check for start of next frame or non-indented section
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            if line.starts_with("SubClassOf:") {
                let expr_str = line.strip_prefix("SubClassOf:").unwrap().trim();
                let superclass = self.parse_class_expression(expr_str)?;
                let subclass = ClassExpression::Class(Class::new(class_iri.clone()));
                axioms.push(Axiom::SubClassOf(SubClassOfAxiom {
                    id: *next_id,
                    subclass,
                    superclass,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("EquivalentTo:") {
                let expr_str = line.strip_prefix("EquivalentTo:").unwrap().trim();
                let class_exprs = self.parse_comma_separated_class_expressions(expr_str)?;
                let mut classes = vec![ClassExpression::Class(Class::new(class_iri.clone()))];
                classes.extend(class_exprs);
                axioms.push(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                    id: *next_id,
                    classes,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("DisjointWith:") {
                let expr_str = line.strip_prefix("DisjointWith:").unwrap().trim();
                let class_exprs = self.parse_comma_separated_class_expressions(expr_str)?;
                let mut classes = vec![ClassExpression::Class(Class::new(class_iri.clone()))];
                classes.extend(class_exprs);
                axioms.push(Axiom::DisjointClasses(DisjointClassesAxiom {
                    id: *next_id,
                    classes,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("DisjointUnionOf:") {
                let expr_str = line.strip_prefix("DisjointUnionOf:").unwrap().trim();
                let disjoint_classes =
                    self.parse_comma_separated_class_expressions(expr_str)?;
                axioms.push(Axiom::DisjointUnion(DisjointUnionAxiom {
                    id: *next_id,
                    class: ClassExpression::Class(Class::new(class_iri.clone())),
                    disjoint_classes,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("HasKey:") {
                let expr_str = line.strip_prefix("HasKey:").unwrap().trim();
                let (obj_props, data_props) = self.parse_has_key_properties(expr_str)?;
                axioms.push(Axiom::HasKey(HasKeyAxiom {
                    id: *next_id,
                    class: ClassExpression::Class(Class::new(class_iri.clone())),
                    object_properties: obj_props,
                    data_properties: data_props,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse an object property frame
    fn parse_object_property_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let prop_iri = self.resolve_iri(name)?;
        let prop = ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: prop_iri.clone(),
        });
        let mut axioms: Vec<Axiom> = Vec::new();

        // Declaration
        axioms.push(Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::ObjectProperty(prop_iri.clone()),
        }));

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            if line.starts_with("Domain:") {
                let expr_str = line.strip_prefix("Domain:").unwrap().trim();
                let domain = self.parse_class_expression(expr_str)?;
                axioms.push(Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
                    id: *next_id,
                    property: prop.clone(),
                    domain,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("Range:") {
                let expr_str = line.strip_prefix("Range:").unwrap().trim();
                let range = self.parse_class_expression(expr_str)?;
                axioms.push(Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
                    id: *next_id,
                    property: prop.clone(),
                    range,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("SubPropertyOf:") {
                let expr_str = line.strip_prefix("SubPropertyOf:").unwrap().trim();
                let super_prop = self.parse_property_expression(expr_str)?;
                axioms.push(Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                    id: *next_id,
                    sub_property: prop.clone(),
                    super_property: super_prop,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("EquivalentTo:") {
                let expr_str = line.strip_prefix("EquivalentTo:").unwrap().trim();
                let props = self.parse_comma_separated_property_expressions(expr_str)?;
                let mut all_props = vec![prop.clone()];
                all_props.extend(props);
                axioms.push(Axiom::EquivalentObjectProperties(
                    EquivalentObjectPropertiesAxiom {
                        id: *next_id,
                        properties: all_props,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("DisjointWith:") {
                let expr_str = line.strip_prefix("DisjointWith:").unwrap().trim();
                let props = self.parse_comma_separated_property_expressions(expr_str)?;
                let mut all_props = vec![prop.clone()];
                all_props.extend(props);
                axioms.push(Axiom::DisjointObjectProperties(
                    DisjointObjectPropertiesAxiom {
                        id: *next_id,
                        properties: all_props,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("InverseOf:") {
                let expr_str = line.strip_prefix("InverseOf:").unwrap().trim();
                let inverse_prop = self.parse_property_expression(expr_str)?;
                axioms.push(Axiom::InverseObjectProperties(
                    InverseObjectPropertiesAxiom {
                        id: *next_id,
                        property1: prop.clone(),
                        property2: inverse_prop,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("SubPropertyChain:") {
                let expr_str = line
                    .strip_prefix("SubPropertyChain:")
                    .unwrap()
                    .trim();
                let props = self.parse_chain_property_expressions(expr_str)?;
                let chain =
                    ObjectPropertyExpression::PropertyChain(props);
                axioms.push(Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                    id: *next_id,
                    sub_property: chain,
                    super_property: prop.clone(),
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("Characteristics:") {
                let char_str = line
                    .strip_prefix("Characteristics:")
                    .unwrap()
                    .trim();
                for chr in char_str.split(',') {
                    let chr = chr.trim();
                    match chr {
                        "Functional" => {
                            axioms.push(Axiom::FunctionalObjectProperty(
                                FunctionalObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "InverseFunctional" => {
                            axioms.push(Axiom::InverseFunctionalObjectProperty(
                                InverseFunctionalObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "Reflexive" => {
                            axioms.push(Axiom::ReflexiveObjectProperty(
                                ReflexiveObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "Irreflexive" => {
                            axioms.push(Axiom::IrreflexiveObjectProperty(
                                IrreflexiveObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "Symmetric" => {
                            axioms.push(Axiom::SymmetricObjectProperty(
                                SymmetricObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "Asymmetric" => {
                            axioms.push(Axiom::AsymmetricObjectProperty(
                                AsymmetricObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        "Transitive" => {
                            axioms.push(Axiom::TransitiveObjectProperty(
                                TransitiveObjectPropertyAxiom {
                                    id: *next_id,
                                    property: prop.clone(),
                                    annotations: Vec::new(),
                                },
                            ));
                            *next_id += 1;
                        }
                        c if !c.is_empty() => {
                            return Err(OxidowlError::ParseError(format!(
                                "Unknown property characteristic: {c}"
                            )));
                        }
                        _ => {}
                    }
                }
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse a data property frame
    fn parse_data_property_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let prop_iri = self.resolve_iri(name)?;
        let prop = crate::ontology::DataPropertyExpression::DataProperty(
            crate::ontology::DataProperty {
                iri: prop_iri.clone(),
            },
        );
        let mut axioms: Vec<Axiom> = Vec::new();

        axioms.push(Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::DataProperty(prop_iri.clone()),
        }));

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            if line.starts_with("Domain:") {
                let expr_str = line.strip_prefix("Domain:").unwrap().trim();
                let domain = self.parse_class_expression(expr_str)?;
                axioms.push(Axiom::DataPropertyDomain(
                    crate::ontology::axioms::DataPropertyDomainAxiom {
                        id: *next_id,
                        property: prop.clone(),
                        domain,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("Range:") {
                let expr_str = line.strip_prefix("Range:").unwrap().trim();
                let range = self.parse_data_range(expr_str)?;
                axioms.push(Axiom::DataPropertyRange(
                    crate::ontology::axioms::DataPropertyRangeAxiom {
                        id: *next_id,
                        property: prop.clone(),
                        range,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("SubPropertyOf:") {
                let expr_str = line.strip_prefix("SubPropertyOf:").unwrap().trim();
                let super_prop = self.parse_data_property_expression(expr_str)?;
                axioms.push(Axiom::SubDataPropertyOf(
                    crate::ontology::axioms::SubDataPropertyOfAxiom {
                        id: *next_id,
                        sub_property: prop.clone(),
                        super_property: super_prop,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("EquivalentTo:") {
                let expr_str = line.strip_prefix("EquivalentTo:").unwrap().trim();
                let dps = self.parse_comma_separated_data_property_expressions(expr_str)?;
                let mut all = vec![prop.clone()];
                all.extend(dps);
                axioms.push(Axiom::EquivalentDataProperties(
                    crate::ontology::axioms::EquivalentDataPropertiesAxiom {
                        id: *next_id,
                        properties: all,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("DisjointWith:") {
                let expr_str = line.strip_prefix("DisjointWith:").unwrap().trim();
                let dps = self.parse_comma_separated_data_property_expressions(expr_str)?;
                let mut all = vec![prop.clone()];
                all.extend(dps);
                axioms.push(Axiom::DisjointDataProperties(
                    crate::ontology::axioms::DisjointDataPropertiesAxiom {
                        id: *next_id,
                        properties: all,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("Characteristics:") {
                let char_str = line.strip_prefix("Characteristics:").unwrap().trim();
                for chr in char_str.split(',') {
                    let chr = chr.trim();
                    if chr == "Functional" {
                        axioms.push(Axiom::FunctionalDataProperty(
                            crate::ontology::axioms::FunctionalDataPropertyAxiom {
                                id: *next_id,
                                property: prop.clone(),
                                annotations: Vec::new(),
                            },
                        ));
                        *next_id += 1;
                    } else if !chr.is_empty() {
                        return Err(OxidowlError::ParseError(format!(
                            "Unknown data property characteristic: {chr}"
                        )));
                    }
                }
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse an individual frame
    fn parse_individual_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let individual_iri = self.resolve_iri(name)?;
        let individual = Individual::Named(crate::ontology::NamedIndividual {
            iri: individual_iri.clone(),
        });
        let mut axioms: Vec<Axiom> = Vec::new();

        axioms.push(Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::NamedIndividual(individual_iri.clone()),
        }));

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            if line.starts_with("Types:") {
                let expr_str = line.strip_prefix("Types:").unwrap().trim();
                let class_exprs = self.parse_comma_separated_class_expressions(expr_str)?;
                for class in class_exprs {
                    axioms.push(Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: *next_id,
                        individual: individual.clone(),
                        class,
                        annotations: Vec::new(),
                    }));
                    *next_id += 1;
                }
            } else if line.starts_with("Facts:") {
                let fact_str = line.strip_prefix("Facts:").unwrap().trim();
                let facts = self.parse_individual_facts(&individual, fact_str)?;
                for fact in facts {
                    axioms.push(fact);
                    *next_id += 1;
                }
            } else if line.starts_with("SameAs:") {
                let expr_str = line.strip_prefix("SameAs:").unwrap().trim();
                let inds = self.parse_comma_separated_individuals(expr_str)?;
                let mut all_inds = vec![individual.clone()];
                all_inds.extend(inds);
                axioms.push(Axiom::SameIndividual(SameIndividualAxiom {
                    id: *next_id,
                    individuals: all_inds,
                    annotations: Vec::new(),
                }));
                *next_id += 1;
            } else if line.starts_with("DifferentFrom:") {
                let expr_str = line.strip_prefix("DifferentFrom:").unwrap().trim();
                let inds = self.parse_comma_separated_individuals(expr_str)?;
                let mut all_inds = vec![individual.clone()];
                all_inds.extend(inds);
                axioms.push(Axiom::DifferentIndividuals(
                    DifferentIndividualsAxiom {
                        id: *next_id,
                        individuals: all_inds,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse a datatype frame
    fn parse_datatype_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        _next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let dtype_iri = self.resolve_iri(name)?;
        let axioms: Vec<Axiom> = vec![Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::Datatype(dtype_iri.clone()),
        })];

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse an annotation property frame
    fn parse_annotation_property_frame(
        &mut self,
        name: &str,
        lines: &[&str],
        index: &mut usize,
        next_id: &mut u64,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let prop_iri = self.resolve_iri(name)?;
        let prop = crate::ontology::AnnotationProperty {
            iri: prop_iri.clone(),
        };
        let mut axioms: Vec<Axiom> = Vec::new();

        axioms.push(Axiom::Declaration(DeclarationAxiom {
            id: 0,
            entity: Entity::AnnotationProperty(prop_iri.clone()),
        }));

        *index += 1;

        while *index < lines.len() {
            let line = lines[*index].trim();
            if line.is_empty() {
                *index += 1;
                continue;
            }
            if line.starts_with("Class:")
                || line.starts_with("ObjectProperty:")
                || line.starts_with("DataProperty:")
                || line.starts_with("Individual:")
                || line.starts_with("Datatype:")
                || line.starts_with("AnnotationProperty:")
                || line.starts_with("Prefix:")
                || line.starts_with("Ontology:")
                || line.starts_with("Import:")
            {
                break;
            }

            if line.starts_with("Domain:") {
                let expr_str = line.strip_prefix("Domain:").unwrap().trim();
                let domain = self.parse_class_expression(expr_str)?;
                axioms.push(Axiom::AnnotationPropertyDomain(
                    AnnotationPropertyDomainAxiom {
                        id: *next_id,
                        property: prop.clone(),
                        domain,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            } else if line.starts_with("Range:") {
                let expr_str = line.strip_prefix("Range:").unwrap().trim();
                let range = self.parse_data_range(expr_str)?;
                axioms.push(Axiom::AnnotationPropertyRange(
                    AnnotationPropertyRangeAxiom {
                        id: *next_id,
                        property: prop.clone(),
                        range,
                        annotations: Vec::new(),
                    },
                ));
                *next_id += 1;
            }

            *index += 1;
        }

        Ok(axioms)
    }

    /// Parse a Manchester syntax class expression into `ClassExpression`
    pub fn parse_class_expression(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ClassExpression, OxidowlError> {
        let expr = expr.trim();
        self.parse_class_expr_internal(expr)
    }

    /// Internal recursive parser for class expressions
    fn parse_class_expr_internal(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ClassExpression, OxidowlError> {
        let expr = expr.trim();

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return self.parse_class_expr_internal(&expr[1..expr.len() - 1]);
        }

        // Handle "not" (ObjectComplementOf)
        if let Some(stripped) = expr.strip_prefix("not ") {
            let inner = self.parse_class_expr_internal(stripped)?;
            return Ok(crate::ontology::ClassExpression::ObjectComplementOf(
                Box::new(inner),
            ));
        }

        // Handle "and" (ObjectIntersectionOf)
        if let Some(and_pos) = self.find_top_level_operator(expr, " and ") {
            let left = self.parse_class_expr_internal(&expr[..and_pos])?;
            let right = self.parse_class_expr_internal(&expr[and_pos + 5..])?;
            return Ok(crate::ontology::ClassExpression::ObjectIntersectionOf(
                vec![left, right],
            ));
        }

        // Handle "or" (ObjectUnionOf)
        if let Some(or_pos) = self.find_top_level_operator(expr, " or ") {
            let left = self.parse_class_expr_internal(&expr[..or_pos])?;
            let right = self.parse_class_expr_internal(&expr[or_pos + 4..])?;
            return Ok(crate::ontology::ClassExpression::ObjectUnionOf(vec![
                left, right,
            ]));
        }

        // Handle property restrictions
        if let Some(some_pos) = self.find_top_level_operator(expr, " some ") {
            let property_str = &expr[..some_pos];
            let filler_str = &expr[some_pos + 6..];
            let property = self.parse_property_expression(property_str)?;
            let filler = self.parse_class_expr_internal(filler_str)?;
            return Ok(crate::ontology::ClassExpression::ObjectSomeValuesFrom {
                property,
                filler: Box::new(filler),
            });
        }

        if let Some(only_pos) = self.find_top_level_operator(expr, " only ") {
            let property_str = &expr[..only_pos];
            let filler_str = &expr[only_pos + 6..];
            let property = self.parse_property_expression(property_str)?;
            let filler = self.parse_class_expr_internal(filler_str)?;
            return Ok(crate::ontology::ClassExpression::ObjectAllValuesFrom {
                property,
                filler: Box::new(filler),
            });
        }

        // Handle exact cardinality: "R exactly 3 C"
        if let Some(exactly_pos) = self.find_top_level_operator(expr, " exactly ") {
            let property_str = &expr[..exactly_pos];
            let rest = &expr[exactly_pos + 9..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectExactCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Handle min cardinality: "R min 2 C"
        if let Some(min_pos) = self.find_top_level_operator(expr, " min ") {
            let property_str = &expr[..min_pos];
            let rest = &expr[min_pos + 5..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectMinCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Handle max cardinality: "R max 5 C"
        if let Some(max_pos) = self.find_top_level_operator(expr, " max ") {
            let property_str = &expr[..max_pos];
            let rest = &expr[max_pos + 5..];
            if let Some(space_pos) = rest.find(' ') {
                let card_str = &rest[..space_pos];
                let filler_str = &rest[space_pos + 1..];
                if let Ok(cardinality) = card_str.parse::<u32>() {
                    let property = self.parse_property_expression(property_str)?;
                    let filler = self.parse_class_expr_internal(filler_str)?;
                    return Ok(crate::ontology::ClassExpression::ObjectMaxCardinality {
                        property,
                        cardinality,
                        filler: Box::new(filler),
                    });
                }
            }
        }

        // Default: treat as a simple class name
        let iri = self.resolve_iri(expr)?;
        Ok(crate::ontology::ClassExpression::Class(
            crate::ontology::Class::new(iri),
        ))
    }

    /// Find the position of an operator at the top level (not inside parentheses)
    fn find_top_level_operator(&self, expr: &str, operator: &str) -> Option<usize> {
        let mut depth = 0;
        let chars: Vec<char> = expr.chars().collect();
        let op_chars: Vec<char> = operator.chars().collect();

        for i in 0..chars.len() {
            if chars[i] == '(' {
                depth += 1;
            } else if chars[i] == ')' {
                depth -= 1;
            } else if depth == 0 {
                // Check if operator matches at this position
                if i + op_chars.len() <= chars.len() {
                    let slice: String = chars[i..i + op_chars.len()].iter().collect();
                    if slice == operator {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// Parse property expression (currently just object properties)
    fn parse_property_expression(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::ObjectPropertyExpression, OxidowlError> {
        let iri = self.resolve_iri(expr.trim())?;
        let object_property = crate::ontology::ObjectProperty { iri };
        Ok(crate::ontology::ObjectPropertyExpression::ObjectProperty(
            object_property,
        ))
    }

    /// Parse cardinality restriction (proper implementation)
    pub fn parse_cardinality_restriction(&self, expr: &str) -> Result<String, OxidowlError> {
        // Parse Manchester syntax cardinality restrictions like:
        // "exactly 1", "min 2", "max 5", "some", "only"
        let expr = expr.trim();

        if let Some(stripped) = expr.strip_prefix("exactly ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("exactly_{num}"));
            }
        } else if let Some(stripped) = expr.strip_prefix("min ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("min_{num}"));
            }
        } else if let Some(stripped) = expr.strip_prefix("max ") {
            let num_str = stripped.trim();
            if let Ok(num) = num_str.parse::<u32>() {
                return Ok(format!("max_{num}"));
            }
        } else if expr == "some" {
            return Ok("some_values_from".to_string());
        } else if expr == "only" {
            return Ok("all_values_from".to_string());
        }

        // Default case
        Ok(expr.to_string())
    }

    /// Parse comma-separated class expressions
    fn parse_comma_separated_class_expressions(
        &self,
        input: &str,
    ) -> Result<Vec<ClassExpression>, OxidowlError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input
            .split(',')
            .map(|s| self.parse_class_expression(s.trim()))
            .collect()
    }

    /// Parse comma-separated property expressions
    fn parse_comma_separated_property_expressions(
        &self,
        input: &str,
    ) -> Result<Vec<ObjectPropertyExpression>, OxidowlError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input
            .split(',')
            .map(|s| self.parse_property_expression(s.trim()))
            .collect()
    }

    /// Parse property expressions separated by "o" (for SubPropertyChain)
    fn parse_chain_property_expressions(
        &self,
        input: &str,
    ) -> Result<Vec<ObjectPropertyExpression>, OxidowlError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input
            .split(" o ")
            .map(|s| self.parse_property_expression(s.trim()))
            .collect()
    }

    /// Parse comma-separated individuals (named)
    fn parse_comma_separated_individuals(
        &self,
        input: &str,
    ) -> Result<Vec<Individual>, OxidowlError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input
            .split(',')
            .map(|s| {
                let s = s.trim();
                let iri = self.resolve_iri(s)?;
                Ok(Individual::Named(crate::ontology::NamedIndividual {
                    iri,
                }))
            })
            .collect()
    }

    /// Parse facts (property-value pairs) for individuals
    fn parse_individual_facts(
        &self,
        subject: &Individual,
        input: &str,
    ) -> Result<Vec<Axiom>, OxidowlError> {
        let mut axioms = Vec::new();
        let input = input.trim();

        // Try to split into property and value
        // Format: "prop value" or "not (prop value)"
        if let Some(negated) = input.strip_prefix("not (")
            && let Some(inner) = negated.strip_suffix(')') {
                let inner = inner.trim();
                if let Some(space_pos) = inner.find(' ') {
                    let prop_str = &inner[..space_pos];
                    let value_str = inner[space_pos + 1..].trim();
                    let property = self.parse_property_expression(prop_str)?;
                    let target_iri = self.resolve_iri(value_str)?;
                    let target = Individual::Named(crate::ontology::NamedIndividual {
                        iri: target_iri,
                    });
                    axioms.push(Axiom::NegativeObjectPropertyAssertion(
                        crate::ontology::axioms::NegativeObjectPropertyAssertionAxiom {
                            id: 0,
                            source: subject.clone(),
                            target,
                            property,
                            annotations: Vec::new(),
                        },
                    ));
                    return Ok(axioms);
                }
            }

        if let Some(space_pos) = input.find(' ') {
            let prop_str = &input[..space_pos];
            let value_str = input[space_pos + 1..].trim();
            // Try parsing as object property assertion first
            let property = self.parse_property_expression(prop_str)?;
            // Check if value looks like an individual or a literal
            let value_trimmed = value_str.trim();
            if value_trimmed.starts_with('"') {
                // It's a literal - data property assertion
                let lit = self.parse_literal(value_trimmed)?;
                let dp = crate::ontology::DataPropertyExpression::DataProperty(
                    crate::ontology::DataProperty {
                        iri: self.resolve_iri(prop_str)?,
                    },
                );
                axioms.push(Axiom::DataPropertyAssertion(
                    crate::ontology::axioms::DataPropertyAssertionAxiom {
                        id: 0,
                        individual: subject.clone(),
                        property: dp,
                        value: lit,
                        annotations: Vec::new(),
                    },
                ));
            } else {
                // Object property assertion
                let target_iri = self.resolve_iri(value_trimmed)?;
                let target = Individual::Named(crate::ontology::NamedIndividual {
                    iri: target_iri,
                });
                axioms.push(Axiom::ObjectPropertyAssertion(
                    ObjectPropertyAssertionAxiom {
                        id: 0,
                        source: subject.clone(),
                        target,
                        property,
                        annotations: Vec::new(),
                    },
                ));
            }
        }

        Ok(axioms)
    }

    /// Parse a literal value (quoted string)
    fn parse_literal(
        &self,
        input: &str,
    ) -> Result<crate::ontology::Literal, OxidowlError> {
        let input = input.trim();
        // Format: "value" or "value"@lang or "value"^^<datatype>
        if let Some(rest) = input.strip_prefix('"') {
            if let Some(quote_pos) = rest.find('"') {
                let value = &rest[..quote_pos];
                let suffix = rest[quote_pos + 1..].trim();
                if let Some(lang_suffix) = suffix.strip_prefix('@') {
                    Ok(crate::ontology::Literal::with_language(
                        value.to_string(),
                        lang_suffix.to_string(),
                    ))
                } else if let Some(dt_suffix) = suffix.strip_prefix("^^") {
                    let dt = self.resolve_iri(dt_suffix.trim())?;
                    Ok(crate::ontology::Literal::with_datatype(
                        value.to_string(),
                        dt,
                    ))
                } else {
                    Ok(crate::ontology::Literal::new(value.to_string()))
                }
            } else {
                Err(OxidowlError::ParseError(format!(
                    "Unterminated string literal: {input}"
                )))
            }
        } else {
            Err(OxidowlError::ParseError(format!(
                "Not a literal: {input}"
            )))
        }
    }

    /// Parse HasKey properties (comma-separated mixed object/data properties)
    fn parse_has_key_properties(
        &self,
        input: &str,
    ) -> Result<
        (
            Vec<ObjectPropertyExpression>,
            Vec<crate::ontology::DataPropertyExpression>,
        ),
        OxidowlError,
    > {
        let mut obj_props = Vec::new();
        let data_props = Vec::new();

        if input.is_empty() {
            return Ok((obj_props, data_props));
        }

        for part in input.split(',') {
            let part = part.trim();
            // Simple heuristic: if it looks like a data property prefix, treat as data property
            // Otherwise default to object property
            let iri = self.resolve_iri(part)?;
            // Default to object property
            obj_props.push(ObjectPropertyExpression::ObjectProperty(
                ObjectProperty { iri },
            ));
        }

        Ok((obj_props, data_props))
    }

    /// Parse a data property expression
    fn parse_data_property_expression(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::DataPropertyExpression, OxidowlError> {
        let iri = self.resolve_iri(expr.trim())?;
        Ok(crate::ontology::DataPropertyExpression::DataProperty(
            crate::ontology::DataProperty { iri },
        ))
    }

    /// Parse comma-separated data property expressions
    fn parse_comma_separated_data_property_expressions(
        &self,
        input: &str,
    ) -> Result<Vec<crate::ontology::DataPropertyExpression>, OxidowlError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input
            .split(',')
            .map(|s| self.parse_data_property_expression(s.trim()))
            .collect()
    }

    /// Parse a data range (simple datatype IRI)
    fn parse_data_range(
        &self,
        expr: &str,
    ) -> Result<crate::ontology::DataRange, OxidowlError> {
        let iri = self.resolve_iri(expr.trim())?;
        Ok(crate::ontology::DataRange::Datatype(iri))
    }

    /// Resolve IRI from prefixed name or full IRI
    fn resolve_iri(&self, name: &str) -> Result<crate::ontology::IRI, OxidowlError> {
        // Handle full IRIs in angle brackets
        if name.starts_with('<') && name.ends_with('>') {
            return Ok(crate::ontology::IRI::new(&name[1..name.len() - 1]));
        }

        // Handle prefixed names
        if name.contains(':') {
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let local_name = parts[1];

                if let Some(namespace) = self.prefixes.get(prefix) {
                    return Ok(crate::ontology::IRI::new(&format!(
                        "{namespace}{local_name}"
                    )));
                }
            }
        }

        // Default to treating as local name with no namespace
        Ok(crate::ontology::IRI::new(name))
    }
}

impl Default for ManchesterParser {
    fn default() -> Self {
        Self::new(ManchesterParserConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_parsing() {
        let mut parser = ManchesterParser::default();
        parser
            .parse_prefix_declaration("Prefix: ex: <http://example.org/>")
            .expect("Failed to parse Manchester syntax prefix declaration");

        assert_eq!(
            parser
                .prefixes
                .get("ex")
                .expect("Failed to get namespace prefix from parser"),
            "http://example.org/"
        );
    }

    #[test]
    fn test_class_expression_parsing() {
        let parser = ManchesterParser::default();

        // Test simple class
        let expr = parser
            .parse_class_expression("Person")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::Class(_) => {}
            _ => panic!("Expected Class variant"),
        }

        // Test intersection
        let expr = parser
            .parse_class_expression("Person and Student")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::ObjectIntersectionOf(_) => {}
            _ => panic!("Expected ObjectIntersectionOf variant"),
        }

        // Test some restriction
        let expr = parser
            .parse_class_expression("hasChild some Person")
            .expect("Failed to parse Manchester syntax class expression");
        match expr {
            crate::ontology::ClassExpression::ObjectSomeValuesFrom { .. } => {}
            _ => panic!("Expected ObjectSomeValuesFrom variant"),
        }
    }

    #[test]
    fn test_manchester_ontology_parsing() {
        let manchester_content = r#"
Prefix: ex: <http://example.org/>

Class: ex:Person

Class: ex:Student
"#;

        let mut parser = ManchesterParser::default();
        let ontology = parser
            .parse_string(manchester_content)
            .expect("Failed to parse Manchester syntax ontology");

        // Each class frame now produces a Declaration axiom
        assert_eq!(ontology.axioms().len(), 2);
        for axiom in ontology.axioms() {
            assert!(matches!(axiom, crate::ontology::axioms::Axiom::Declaration(_)));
        }
    }

    #[test]
    fn test_full_manchester_ontology() {
        let manchester_content = r#"
Prefix: ex: <http://example.org/>

Ontology: <http://example.org/onto>

Class: ex:Person
    SubClassOf: ex:Animal
    EquivalentTo: ex:Human and ex:Organism

Class: ex:Student
    SubClassOf: ex:Person
    DisjointWith: ex:Teacher, ex:Staff

ObjectProperty: ex:hasChild
    Domain: ex:Person
    Range: ex:Person
    Characteristics: Transitive, Asymmetric

ObjectProperty: ex:hasParent
    SubPropertyOf: ex:hasRelative
    InverseOf: ex:hasChild

Individual: ex:Alice
    Types: ex:Person
    Facts: ex:hasChild ex:Bob

Individual: ex:Bob
    Types: ex:Student
    SameAs: ex:Robert

AnnotationProperty: ex:label
    Domain: ex:Thing
    Range: xsd:string
"#;

        let mut parser = ManchesterParser::default();
        let ontology = parser
            .parse_string(manchester_content)
            .expect("Failed to parse Manchester syntax ontology");

        let axioms = ontology.axioms();
        assert!(axioms.len() > 10, "Expected many axioms, got {}", axioms.len());

        // Check for declarations
        let declarations: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::Declaration(_)))
            .collect();
        assert!(!declarations.is_empty(), "Expected at least one declaration");

        // Check for SubClassOf axioms
        let subclassof: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::SubClassOf(_)))
            .collect();
        assert!(!subclassof.is_empty(), "Expected at least one SubClassOf");

        // Check for EquivalentClasses axioms
        let equiv: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::EquivalentClasses(_)))
            .collect();
        assert!(!equiv.is_empty(), "Expected at least one EquivalentClasses");

        // Check for DisjointClasses axioms
        let disjoint: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::DisjointClasses(_)))
            .collect();
        assert!(!disjoint.is_empty(), "Expected at least one DisjointClasses");

        // Check for property characteristic axioms
        let transitive: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::TransitiveObjectProperty(_)))
            .collect();
        assert!(!transitive.is_empty(), "Expected at least one TransitiveObjectProperty");

        let asymmetric: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::AsymmetricObjectProperty(_)))
            .collect();
        assert!(!asymmetric.is_empty(), "Expected at least one AsymmetricObjectProperty");

        // Check for InverseObjectProperties
        let inverse: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::InverseObjectProperties(_)))
            .collect();
        assert!(!inverse.is_empty(), "Expected at least one InverseObjectProperties");

        // Check for ClassAssertion (Types:)
        let class_assert: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::ClassAssertion(_)))
            .collect();
        assert!(!class_assert.is_empty(), "Expected at least one ClassAssertion");

        // Check for ObjectPropertyAssertion (Facts:)
        let prop_assert: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::ObjectPropertyAssertion(_)))
            .collect();
        assert!(!prop_assert.is_empty(), "Expected at least one ObjectPropertyAssertion");

        // Check for SameIndividual
        let same: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::SameIndividual(_)))
            .collect();
        assert!(!same.is_empty(), "Expected at least one SameIndividual");

        // Check for AnnotationPropertyDomain
        let ap_domain: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::AnnotationPropertyDomain(_)))
            .collect();
        assert!(!ap_domain.is_empty(), "Expected at least one AnnotationPropertyDomain");

        // Check for AnnotationPropertyRange
        let ap_range: Vec<_> = axioms
            .iter()
            .filter(|a| matches!(a, crate::ontology::axioms::Axiom::AnnotationPropertyRange(_)))
            .collect();
        assert!(!ap_range.is_empty(), "Expected at least one AnnotationPropertyRange");
    }
}
