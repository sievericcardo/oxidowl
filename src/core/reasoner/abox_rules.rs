//! Built-in OWL 2 ABox classification SPARQL INSERT-WHERE rules.
//!
//! Expose the rule set so external projects that manage their own Oxigraph store
//! can run exactly the same rules without duplicating the queries:
//!
//! ```rust,ignore
//! for _ in 0..20 {
//!     for rule in oxidowl::abox_classification_rules() {
//!         store.execute_update(rule)?;
//!     }
//! }
//! ```
//!
//! The rules cover:
//! 1. `hasValue` restriction (`C ≡ (p hasValue v)`) — via `equivalentClass`
//! 2. `someValuesFrom` restriction (`C ≡ (p someValuesFrom D)`) — via `equivalentClass`
//! 3.  Named-class equivalence (`C ≡ D`) — both directions
//!     3a. `D` instances become `C` instances
//!     3b. `C` instances become `D` instances
//! 4. `intersectionOf(DataSomeValuesFrom(dp, xsd:double range), DataHasValue(hvp, hv))`
//! 5. Pure named-class `intersectionOf`
//! 6. `rdfs:subClassOf` propagation
//! 7. `rdfs:domain` propagation
//! 8. `rdfs:range` propagation
//! 9. `owl:unionOf` (OWL 2 RL cls-uni) — union member → union class
//! 10. `intersectionOf` member propagation (OWL 2 RL cls-int2) — intersection class → each named member
//!     11a/11b. `owl:sameAs` type propagation (OWL 2 RL eq-rep-s/o) — both directions
//! 12. `hasValue` restriction via `rdfs:subClassOf` (OWL 2 RL cls-hv1 general)
//! 13. `someValuesFrom` restriction via `rdfs:subClassOf` (OWL 2 RL cls-svf1 general)
//! 14. `allValuesFrom` restriction via `rdfs:subClassOf` (OWL 2 RL cls-avf1 general) — three patterns:
//!     14a. `allValuesFrom` via `equivalentClass` (OWL 2 RL cls-avf)
//!     14b. `allValuesFrom` via `rdfs:subClassOf` (OWL 2 RL cls-avf general form)
//! 15. `rdfs:subPropertyOf` propagation (OWL 2 RL prp-spo1) — property enabler
//! 16. `owl:TransitiveProperty` propagation (OWL 2 RL prp-trp)
//! 17. `owl:SymmetricProperty` propagation (OWL 2 RL prp-symp)
//!     18a/18b. `owl:inverseOf` propagation (OWL 2 RL prp-inv1/inv2) — both directions

/// Returns the complete set of built-in OWL 2 ABox classification SPARQL
/// INSERT-WHERE rules that oxidowl applies during `run_sparql_abox_classification`.
pub fn abox_classification_rules() -> &'static [&'static str] {
    &[
        // Rule 1: hasValue restriction — C ≡ (p hasValue v)
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?R . \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty> ?P ; \
             <http://www.w3.org/2002/07/owl#hasValue>    ?V . \
          ?x ?P ?V . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 2: someValuesFrom restriction — C ≡ (p someValuesFrom D)
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?R . \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty>    ?P ; \
             <http://www.w3.org/2002/07/owl#someValuesFrom> ?D . \
          ?x ?P ?y . \
          ?y a ?D . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 3a: named class equivalence C ≡ D → D instances become C instances
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?D . \
          FILTER(!isBlank(?D)) \
          ?x a ?D . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 3b: named class equivalence C ≡ D → C instances become D instances
        "INSERT { ?x a ?D } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?D . \
          FILTER(!isBlank(?D)) \
          ?x a ?C . \
          FILTER NOT EXISTS { ?x a ?D } \
        }",
        // Rule 4: intersectionOf( DataSomeValuesFrom(dp, xsd:double range), DataHasValue(hvp, hv) )
        //   Only fires for pure 2-item (double + hasValue) intersections.  Guards prevent firing
        //   for 3-item (integer+double+hasValue) Overheating/Underheating classes and 4-item
        //   (integer+double+double+hasValue) Operational classes, which are handled by Rules 4b/4d.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_range . \
          ?r_range <http://www.w3.org/2002/07/owl#onProperty>    ?dp . \
          ?r_range <http://www.w3.org/2002/07/owl#someValuesFrom> ?dr . \
          ?dr <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dr <http://www.w3.org/2002/07/owl#withRestrictions> ?flist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_hv . \
          ?r_hv <http://www.w3.org/2002/07/owl#onProperty> ?hvp . \
          ?r_hv <http://www.w3.org/2002/07/owl#hasValue>    ?hv . \
          FILTER(?r_range != ?r_hv) \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_int_ex . \
            ?r_int_ex <http://www.w3.org/2002/07/owl#someValuesFrom> ?di_ex . \
            ?di_ex <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#integer> . \
          } \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r_dbl2 . \
            ?r_dbl2 <http://www.w3.org/2002/07/owl#onProperty> ?dp . \
            ?r_dbl2 <http://www.w3.org/2002/07/owl#someValuesFrom> ?dd2 . \
            ?dd2 <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
            FILTER(?r_dbl2 != ?r_range) \
          } \
          ?x ?dp  ?numval . \
          ?x ?hvp ?hv . \
          OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_mi . \
                     ?f_mi <http://www.w3.org/2001/XMLSchema#minInclusive> ?minI . } \
          OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_me . \
                     ?f_me <http://www.w3.org/2001/XMLSchema#minExclusive> ?minE . } \
          OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_xi . \
                     ?f_xi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?maxI . } \
          OPTIONAL { ?flist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f_xe . \
                     ?f_xe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?maxE . } \
          FILTER((!BOUND(?minI) || ?numval >= ?minI) && \
                 (!BOUND(?minE) || ?numval >  ?minE) && \
                 (!BOUND(?maxI) || ?numval <= ?maxI) && \
                 (!BOUND(?maxE) || ?numval <  ?maxE)) \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 4b: intersectionOf( DataSomeValuesFrom(ip, xsd:integer range),
        //                           DataSomeValuesFrom(dp, xsd:double range),
        //                           DataHasValue(hvp, hv) )
        //   Handles Overheating, Underheating, AND Maintenance pump classes (3-item intersection
        //   with exactly one integer restriction, one double restriction, and one hasValue
        //   restriction).  All combinations of integer facets are supported:
        //   - maxExclusive / maxInclusive (Overheating, Underheating: life < threshold)
        //   - minInclusive / minExclusive (Maintenance: life >= threshold)
        //   FILTER NOT EXISTS prevents firing when a second double restriction exists for the
        //   same property (that 4-item Operational pattern is handled by Rule 4d instead).
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4b_int . \
          ?r4b_int <http://www.w3.org/2002/07/owl#onProperty>    ?ip . \
          ?r4b_int <http://www.w3.org/2002/07/owl#someValuesFrom> ?di . \
          ?di <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#integer> . \
          ?di <http://www.w3.org/2002/07/owl#withRestrictions> ?fi . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4b_dbl . \
          ?r4b_dbl <http://www.w3.org/2002/07/owl#onProperty>    ?dp . \
          ?r4b_dbl <http://www.w3.org/2002/07/owl#someValuesFrom> ?dd . \
          ?dd <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dd <http://www.w3.org/2002/07/owl#withRestrictions> ?fd . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4b_hv . \
          ?r4b_hv <http://www.w3.org/2002/07/owl#onProperty> ?hvp . \
          ?r4b_hv <http://www.w3.org/2002/07/owl#hasValue>    ?hv . \
          FILTER(?r4b_int != ?r4b_dbl && ?r4b_int != ?r4b_hv && ?r4b_dbl != ?r4b_hv) \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4b_dbl2 . \
            ?r4b_dbl2 <http://www.w3.org/2002/07/owl#onProperty>    ?dp . \
            ?r4b_dbl2 <http://www.w3.org/2002/07/owl#someValuesFrom> ?dd2 . \
            ?dd2 <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
            FILTER(?r4b_dbl2 != ?r4b_dbl) \
          } \
          ?x ?ip ?intval . \
          ?x ?dp ?dblval . \
          ?x ?hvp ?hv . \
          OPTIONAL { ?fi (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_ixe . \
                     ?f4b_ixe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?ixE . } \
          OPTIONAL { ?fi (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_ixi . \
                     ?f4b_ixi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?ixI . } \
          OPTIONAL { ?fi (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_ini . \
                     ?f4b_ini <http://www.w3.org/2001/XMLSchema#minInclusive> ?inI . } \
          OPTIONAL { ?fi (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_ine . \
                     ?f4b_ine <http://www.w3.org/2001/XMLSchema#minExclusive> ?inE . } \
          OPTIONAL { ?fd (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_dmi . \
                     ?f4b_dmi <http://www.w3.org/2001/XMLSchema#minInclusive> ?dminI . } \
          OPTIONAL { ?fd (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_dme . \
                     ?f4b_dme <http://www.w3.org/2001/XMLSchema#minExclusive> ?dminE . } \
          OPTIONAL { ?fd (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_dxi . \
                     ?f4b_dxi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?dmaxI . } \
          OPTIONAL { ?fd (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4b_dxe . \
                     ?f4b_dxe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?dmaxE . } \
          FILTER((!BOUND(?ixE)   || ?intval  < ?ixE)   && \
                 (!BOUND(?ixI)   || ?intval  <= ?ixI)  && \
                 (!BOUND(?inI)   || ?intval  >= ?inI)  && \
                 (!BOUND(?inE)   || ?intval  >  ?inE)  && \
                 (!BOUND(?dminI) || ?dblval  >= ?dminI) && \
                 (!BOUND(?dminE) || ?dblval  >  ?dminE) && \
                 (!BOUND(?dmaxI) || ?dblval  <= ?dmaxI) && \
                 (!BOUND(?dmaxE) || ?dblval  <  ?dmaxE)) \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 4d: intersectionOf( DataSomeValuesFrom(ip, xsd:integer range),
        //                           DataSomeValuesFrom(dp, xsd:double minInclusive lo),
        //                           DataSomeValuesFrom(dp, xsd:double maxInclusive hi),
        //                           DataHasValue(hvp, hv) )
        //   Handles Operational pump classes where temperature must fall in a closed interval
        //   [lo, hi] AND pumpLifeTime must be below a threshold.  Both double bounds must appear
        //   as DISTINCT intersection members for the SAME property.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4d_int . \
          ?r4d_int <http://www.w3.org/2002/07/owl#onProperty>    ?ip . \
          ?r4d_int <http://www.w3.org/2002/07/owl#someValuesFrom> ?di4d . \
          ?di4d <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#integer> . \
          ?di4d <http://www.w3.org/2002/07/owl#withRestrictions> ?fi4d . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4d_lo . \
          ?r4d_lo <http://www.w3.org/2002/07/owl#onProperty>    ?dp4d . \
          ?r4d_lo <http://www.w3.org/2002/07/owl#someValuesFrom> ?dlo . \
          ?dlo <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dlo <http://www.w3.org/2002/07/owl#withRestrictions> ?flo . \
          ?flo (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4d_lo_n . \
          ?f4d_lo_n <http://www.w3.org/2001/XMLSchema#minInclusive> ?loVal . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4d_hi . \
          ?r4d_hi <http://www.w3.org/2002/07/owl#onProperty>    ?dp4d . \
          ?r4d_hi <http://www.w3.org/2002/07/owl#someValuesFrom> ?dhi . \
          ?dhi <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dhi <http://www.w3.org/2002/07/owl#withRestrictions> ?fhi . \
          ?fhi (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4d_hi_n . \
          ?f4d_hi_n <http://www.w3.org/2001/XMLSchema#maxInclusive> ?hiVal . \
          FILTER(?r4d_lo != ?r4d_hi) \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4d_hv . \
          ?r4d_hv <http://www.w3.org/2002/07/owl#onProperty> ?hvp4d . \
          ?r4d_hv <http://www.w3.org/2002/07/owl#hasValue>    ?hv4d . \
          FILTER(?r4d_int != ?r4d_lo && ?r4d_int != ?r4d_hi && ?r4d_int != ?r4d_hv && \
                 ?r4d_lo != ?r4d_hv && ?r4d_hi != ?r4d_hv) \
          ?x ?ip ?intval4d . \
          ?x ?dp4d ?numval4d . \
          ?x ?hvp4d ?hv4d . \
          OPTIONAL { ?fi4d (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4d_ixe . \
                     ?f4d_ixe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?ixE4d . } \
          FILTER((!BOUND(?ixE4d) || ?intval4d < ?ixE4d) && \
                 ?numval4d >= ?loVal && \
                 ?numval4d <= ?hiVal) \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 4e: intersectionOf( DataSomeValuesFrom(dp, xsd:double lo_range),
        //                           DataSomeValuesFrom(dp, xsd:double hi_range),
        //                           DataHasValue(hvp, hv) )
        //   Handles plant Unhealthy state classes (UnhealthyX pattern): a 3-item intersection
        //   with exactly TWO double restrictions on the SAME property (lower + upper bound) and
        //   ONE hasValue restriction on a different property (e.g. familyName).
        //   All combinations of min/maxInclusive and min/maxExclusive facets are supported.
        //   FILTER NOT EXISTS guards exclude integer-bearing patterns (Rules 4b/4d) and
        //   intersections with more than two double restrictions for the same property.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4e_lo . \
          ?r4e_lo <http://www.w3.org/2002/07/owl#onProperty>    ?dp4e . \
          ?r4e_lo <http://www.w3.org/2002/07/owl#someValuesFrom> ?dlo4e . \
          ?dlo4e <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dlo4e <http://www.w3.org/2002/07/owl#withRestrictions> ?flo4e . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4e_hi . \
          ?r4e_hi <http://www.w3.org/2002/07/owl#onProperty>    ?dp4e . \
          ?r4e_hi <http://www.w3.org/2002/07/owl#someValuesFrom> ?dhi4e . \
          ?dhi4e <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
          ?dhi4e <http://www.w3.org/2002/07/owl#withRestrictions> ?fhi4e . \
          FILTER(?r4e_lo != ?r4e_hi) \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4e_hv . \
          ?r4e_hv <http://www.w3.org/2002/07/owl#onProperty> ?hvp4e . \
          ?r4e_hv <http://www.w3.org/2002/07/owl#hasValue>    ?hv4e . \
          FILTER(?r4e_lo != ?r4e_hv && ?r4e_hi != ?r4e_hv) \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4e_dbl3 . \
            ?r4e_dbl3 <http://www.w3.org/2002/07/owl#onProperty>    ?dp4e . \
            ?r4e_dbl3 <http://www.w3.org/2002/07/owl#someValuesFrom> ?dd4e3 . \
            ?dd4e3 <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#double> . \
            FILTER(?r4e_dbl3 != ?r4e_lo && ?r4e_dbl3 != ?r4e_hi) \
          } \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?r4e_int . \
            ?r4e_int <http://www.w3.org/2002/07/owl#someValuesFrom> ?di4e . \
            ?di4e <http://www.w3.org/2002/07/owl#onDatatype> <http://www.w3.org/2001/XMLSchema#integer> . \
          } \
          OPTIONAL { ?flo4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_lomi . \
                     ?f4e_lomi <http://www.w3.org/2001/XMLSchema#minInclusive> ?loMinI4e . } \
          OPTIONAL { ?flo4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_lome . \
                     ?f4e_lome <http://www.w3.org/2001/XMLSchema#minExclusive> ?loMinE4e . } \
          OPTIONAL { ?flo4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_loxi . \
                     ?f4e_loxi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?loMaxI4e . } \
          OPTIONAL { ?flo4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_loxe . \
                     ?f4e_loxe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?loMaxE4e . } \
          OPTIONAL { ?fhi4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_himi . \
                     ?f4e_himi <http://www.w3.org/2001/XMLSchema#minInclusive> ?hiMinI4e . } \
          OPTIONAL { ?fhi4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_hime . \
                     ?f4e_hime <http://www.w3.org/2001/XMLSchema#minExclusive> ?hiMinE4e . } \
          OPTIONAL { ?fhi4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_hixi . \
                     ?f4e_hixi <http://www.w3.org/2001/XMLSchema#maxInclusive> ?hiMaxI4e . } \
          OPTIONAL { ?fhi4e (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?f4e_hixe . \
                     ?f4e_hixe <http://www.w3.org/2001/XMLSchema#maxExclusive> ?hiMaxE4e . } \
          ?x ?dp4e ?numval4e . \
          ?x ?hvp4e ?hv4e . \
          FILTER((!BOUND(?loMinI4e) || ?numval4e >= ?loMinI4e) && \
                 (!BOUND(?loMinE4e) || ?numval4e >  ?loMinE4e) && \
                 (!BOUND(?loMaxI4e) || ?numval4e <= ?loMaxI4e) && \
                 (!BOUND(?loMaxE4e) || ?numval4e <  ?loMaxE4e) && \
                 (!BOUND(?hiMinI4e) || ?numval4e >= ?hiMinI4e) && \
                 (!BOUND(?hiMinE4e) || ?numval4e >  ?hiMinE4e) && \
                 (!BOUND(?hiMaxI4e) || ?numval4e <= ?hiMaxI4e) && \
                 (!BOUND(?hiMaxE4e) || ?numval4e <  ?hiMaxE4e)) \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 5: pure named-class intersectionOf
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m . \
            FILTER(isBlank(?m)) \
          } \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m2 . \
            FILTER NOT EXISTS { ?x a ?m2 } \
          } \
          ?ilist <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?m1 . \
          ?x a ?m1 . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 6: rdfs:subClassOf propagation
        "INSERT { ?x a ?super } \
        WHERE { \
          ?sub <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?super . \
          FILTER(!isBlank(?super)) \
          ?x a ?sub . \
          FILTER NOT EXISTS { ?x a ?super } \
        }",
        // Rule 7: rdfs:domain propagation
        "INSERT { ?x a ?C } WHERE { ?x ?P ?y . ?P <http://www.w3.org/2000/01/rdf-schema#domain> ?C . FILTER NOT EXISTS { ?x a ?C } FILTER(isIRI(?C)) }",
        // Rule 8: rdfs:range propagation
        "INSERT { ?y a ?C } WHERE { ?x ?P ?y . ?P <http://www.w3.org/2000/01/rdf-schema#range> ?C . FILTER NOT EXISTS { ?y a ?C } FILTER(isIRI(?C)) }",
        // Rule 9: owl:unionOf classification (OWL 2 RL cls-uni)
        //   C ≡ (A ∪ B ∪ …) — if ?x is an instance of any union member, classify ?x as ?C.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#unionOf> ?ulist . \
          ?ulist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?member . \
          FILTER(!isBlank(?member)) \
          ?x a ?member . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 10: intersectionOf member propagation (OWL 2 RL cls-int2)
        //   C ≡ intersectionOf(A, B, …) — if ?x a ?C, propagate membership to every named
        //   class in the intersection list (enables further subClassOf chains to fire).
        "INSERT { ?x a ?member } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?member . \
          FILTER(!isBlank(?member)) \
          ?x a ?C . \
          FILTER NOT EXISTS { ?x a ?member } \
        }",
        // Rule 11a: owl:sameAs type propagation — forward (OWL 2 RL eq-rep-s)
        //   If ?x owl:sameAs ?y and ?y a ?C, then ?x a ?C.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#sameAs> ?y . \
          ?y a ?C . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 11b: owl:sameAs type propagation — backward (OWL 2 RL eq-rep-o)
        //   If ?x owl:sameAs ?y and ?x a ?C, then ?y a ?C.
        "INSERT { ?y a ?C } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#sameAs> ?y . \
          ?x a ?C . \
          FILTER NOT EXISTS { ?y a ?C } \
        }",
        // Rule 12: hasValue restriction via rdfs:subClassOf (OWL 2 RL cls-hv1 general form)
        //   If a blank-node restriction (P hasValue V) is declared as a subclass of ?C, and
        //   ?x P V holds, classify ?x as ?C.  Complements Rule 1 which uses equivalentClass.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty> ?P ; \
             <http://www.w3.org/2002/07/owl#hasValue>    ?V ; \
             <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?C . \
          FILTER(!isBlank(?C)) \
          ?x ?P ?V . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 13: someValuesFrom restriction via rdfs:subClassOf (OWL 2 RL cls-svf1 general)
        //   If a blank-node restriction (P someValuesFrom D) is declared as a subclass of ?C,
        //   and ?x ?P ?y with ?y a ?D, classify ?x as ?C.  Complements Rule 2.
        "INSERT { ?x a ?C } \
        WHERE { \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty>    ?P ; \
             <http://www.w3.org/2002/07/owl#someValuesFrom> ?D ; \
             <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?C . \
          FILTER(!isBlank(?C)) \
          FILTER(!isBlank(?D)) \
          ?x ?P ?y . \
          ?y a ?D . \
          FILTER NOT EXISTS { ?x a ?C } \
        }",
        // Rule 14a: allValuesFrom via equivalentClass (OWL 2 RL cls-avf)
        //   C ≡ restriction(P allValuesFrom D) — if ?x a ?C and ?x ?P ?y, classify ?y as ?D.
        "INSERT { ?y a ?D } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?R . \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty>    ?P ; \
             <http://www.w3.org/2002/07/owl#allValuesFrom> ?D . \
          FILTER(!isBlank(?D)) \
          ?x a ?C . \
          ?x ?P ?y . \
          FILTER NOT EXISTS { ?y a ?D } \
        }",
        // Rule 14b: allValuesFrom via rdfs:subClassOf (OWL 2 RL cls-avf general form)
        //   C subClassOf restriction(P allValuesFrom D) is the common OWL pattern;
        //   if ?x a ?C and ?x ?P ?y, classify ?y as ?D.
        "INSERT { ?y a ?D } \
        WHERE { \
          ?C <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?R . \
          ?R <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> ; \
             <http://www.w3.org/2002/07/owl#onProperty>    ?P ; \
             <http://www.w3.org/2002/07/owl#allValuesFrom> ?D . \
          FILTER(!isBlank(?D)) \
          ?x a ?C . \
          ?x ?P ?y . \
          FILTER NOT EXISTS { ?y a ?D } \
        }",
        // Rule 15: rdfs:subPropertyOf propagation (OWL 2 RL prp-spo1)
        //   If ?subP rdfs:subPropertyOf ?superP and ?x ?subP ?y, assert ?x ?superP ?y.
        //   This enables domain/range/someValuesFrom rules to fire via the super-property.
        "INSERT { ?x ?superP ?y } \
        WHERE { \
          ?subP <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?superP . \
          FILTER(!isBlank(?superP)) \
          ?x ?subP ?y . \
          FILTER NOT EXISTS { ?x ?superP ?y } \
        }",
        // Rule 16: owl:TransitiveProperty propagation (OWL 2 RL prp-trp)
        //   If ?P a owl:TransitiveProperty and ?x ?P ?y and ?y ?P ?z, assert ?x ?P ?z.
        "INSERT { ?x ?P ?z } \
        WHERE { \
          ?P <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#TransitiveProperty> . \
          ?x ?P ?y . \
          ?y ?P ?z . \
          FILTER(?x != ?z) \
          FILTER NOT EXISTS { ?x ?P ?z } \
        }",
        // Rule 17: owl:SymmetricProperty propagation (OWL 2 RL prp-symp)
        //   If ?P a owl:SymmetricProperty and ?x ?P ?y, assert ?y ?P ?x.
        "INSERT { ?y ?P ?x } \
        WHERE { \
          ?P <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#SymmetricProperty> . \
          ?x ?P ?y . \
          FILTER NOT EXISTS { ?y ?P ?x } \
        }",
        // Rule 18a: owl:inverseOf propagation — forward (OWL 2 RL prp-inv1)
        //   If ?P owl:inverseOf ?Q and ?x ?P ?y, assert ?y ?Q ?x.
        "INSERT { ?y ?Q ?x } \
        WHERE { \
          ?P <http://www.w3.org/2002/07/owl#inverseOf> ?Q . \
          ?x ?P ?y . \
          FILTER NOT EXISTS { ?y ?Q ?x } \
        }",
        // Rule 18b: owl:inverseOf propagation — backward (OWL 2 RL prp-inv2)
        //   If ?P owl:inverseOf ?Q and ?y ?Q ?x, assert ?x ?P ?y.
        "INSERT { ?x ?P ?y } \
        WHERE { \
          ?P <http://www.w3.org/2002/07/owl#inverseOf> ?Q . \
          ?y ?Q ?x . \
          FILTER NOT EXISTS { ?x ?P ?y } \
        }",
    ]
}
