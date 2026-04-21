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
//! **Coverage vs. OWL 2 RL (W3C, Tables 4–9)**
//!
//! *Positive classification rules* (INSERT-WHERE, implemented here):
//!
//! | Rule | Description |
//! |------|-------------|
//! | 1    | `cls-hv2` via `equivalentClass` |
//! | 2    | `cls-svf1` via `equivalentClass` |
//! | 3a/b | `cax-eqc1/2` — named-class equivalence, both directions |
//! | 4/4b/4d/4e | Specialised intersection + data-range + `hasValue` patterns |
//! | 5    | `cls-int1` — pure named-class `intersectionOf` via `equivalentClass` |
//! | 6    | `cax-sco` — `rdfs:subClassOf` propagation |
//! | 7    | `prp-dom` — `rdfs:domain` propagation |
//! | 8    | `prp-rng` — `rdfs:range` propagation |
//! | 9    | `cls-uni` — `owl:unionOf` (via `equivalentClass`) |
//! | 10   | `cls-int2` — intersection → member propagation (via `equivalentClass`) |
//! | 11a/b| `eq-rep-s/o` — `owl:sameAs` type propagation, both directions |
//! | 12   | `cls-hv2` via `rdfs:subClassOf` |
//! | 13   | `cls-svf1` via `rdfs:subClassOf` |
//! | 14a/b| `cls-avf` via `equivalentClass` and `rdfs:subClassOf` |
//! | 15   | `prp-spo1` — `rdfs:subPropertyOf` propagation |
//! | 16   | `prp-trp` — `owl:TransitiveProperty` |
//! | 17   | `prp-symp` — `owl:SymmetricProperty` |
//! | 18a/b| `prp-inv1/2` — `owl:inverseOf`, both directions |
//! | 19   | `eq-sym` — `owl:sameAs` symmetry |
//! | 20   | `eq-trans` — `owl:sameAs` transitivity |
//! | 21   | `eq-rep-s` (general) — property-assertion propagation via `sameAs`, subject side |
//! | 22   | `eq-rep-o` (general) — property-assertion propagation via `sameAs`, object side |
//! | 23   | `prp-fp` — `owl:FunctionalProperty` → `owl:sameAs` |
//! | 24   | `prp-ifp` — `owl:InverseFunctionalProperty` → `owl:sameAs` |
//! | 25   | `prp-eqp1` — `owl:equivalentProperty` forward |
//! | 26   | `prp-eqp2` — `owl:equivalentProperty` backward |
//! | 27   | `prp-spo2` — `owl:propertyChainAxiom` (length-2 chains) |
//! | 28   | `cls-svf1` (raw) — `someValuesFrom` → blank-node restriction type |
//! | 29   | `cls-svf2` — `someValuesFrom owl:Thing` |
//! | 30   | `cls-hv1` — `hasValue` restriction → property assertion |
//! | 31   | `cls-hv2` (raw) — `hasValue` → blank-node restriction type |
//! | 32   | `cls-oo` — `owl:oneOf` enumeration → type each member |
//! | 33   | `cls-int1` (general) — direct `owl:intersectionOf` on named class |
//! | 34   | `cls-int2` (general) — direct `owl:intersectionOf` member propagation |
//! | 35   | `cls-uni` (general) — direct `owl:unionOf` on named class |
//! | 36   | `cls-int1` via `equivalentClass` — allows typed blank-node members |
//! | 37   | `scm-sco` — `rdfs:subClassOf` transitivity |
//! | 38a/b| `scm-eqc1` — `owl:equivalentClass` → `rdfs:subClassOf`, both directions |
//! | 39   | `scm-eqc2` — mutual `rdfs:subClassOf` → `owl:equivalentClass` |
//! | 40   | `scm-spo` — `rdfs:subPropertyOf` transitivity |
//! | 41a/b| `scm-eqp1` — `owl:equivalentProperty` → `rdfs:subPropertyOf`, both directions |
//! | 42   | `scm-eqp2` — mutual `rdfs:subPropertyOf` → `owl:equivalentProperty` |
//! | 43   | `scm-dom1` — domain + `rdfs:subClassOf` chain |
//! | 44   | `scm-dom2` — domain + `rdfs:subPropertyOf` chain |
//! | 45   | `scm-rng1` — range + `rdfs:subClassOf` chain |
//! | 46   | `scm-rng2` — range + `rdfs:subPropertyOf` chain |
//! | 47   | `scm-int` — `owl:intersectionOf` → `rdfs:subClassOf` each member |
//! | 48   | `scm-uni` — each union member `rdfs:subClassOf` the union class |
//! | 49   | `scm-svf1` — `someValuesFrom` with filler class subsumption |
//! | 50   | `scm-svf2` — `someValuesFrom` with property subsumption |
//! | 51   | `scm-avf1` — `allValuesFrom` with filler class subsumption |
//! | 52   | `scm-avf2` — `allValuesFrom` with property subsumption (reversed direction) |
//!
//! *Consistency-detection rules* (derive `false`; require separate validation, **not** INSERT-WHERE):
//! `eq-diff1/2/3`, `cls-nothing2`, `cls-com`, `cls-maxc1/2`, `cls-maxqc1–4`,
//! `cax-dw`, `cax-adc`, `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-adp`, `prp-npa1/2`,
//! `prp-key` (derives `owl:sameAs`, but requires arbitrary-length key lists).

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

        // ── Equality rules (OWL 2 RL Table 4) ────────────────────────────────────

        // Rule 19: eq-sym — owl:sameAs symmetry
        "INSERT { ?y <http://www.w3.org/2002/07/owl#sameAs> ?x } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#sameAs> ?y . \
          FILTER(?x != ?y) \
          FILTER NOT EXISTS { ?y <http://www.w3.org/2002/07/owl#sameAs> ?x } \
        }",

        // Rule 20: eq-trans — owl:sameAs transitivity
        "INSERT { ?x <http://www.w3.org/2002/07/owl#sameAs> ?z } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#sameAs> ?y . \
          ?y <http://www.w3.org/2002/07/owl#sameAs> ?z . \
          FILTER(?x != ?z) \
          FILTER NOT EXISTS { ?x <http://www.w3.org/2002/07/owl#sameAs> ?z } \
        }",

        // Rule 21: eq-rep-s (general) — propagate property assertions through sameAs on subject
        //   Excludes rdf:type (covered by Rules 11a/11b) and owl:sameAs itself.
        "INSERT { ?s2 ?p ?o } \
        WHERE { \
          ?s1 <http://www.w3.org/2002/07/owl#sameAs> ?s2 . \
          ?s1 ?p ?o . \
          FILTER(!isBlank(?s2)) \
          FILTER(?p != <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>) \
          FILTER(?p != <http://www.w3.org/2002/07/owl#sameAs>) \
          FILTER NOT EXISTS { ?s2 ?p ?o } \
        }",

        // Rule 22: eq-rep-o (general) — propagate property assertions through sameAs on object
        //   Excludes rdf:type and owl:sameAs.
        "INSERT { ?s ?p ?o2 } \
        WHERE { \
          ?o1 <http://www.w3.org/2002/07/owl#sameAs> ?o2 . \
          ?s ?p ?o1 . \
          FILTER(!isBlank(?o2)) \
          FILTER(?p != <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>) \
          FILTER(?p != <http://www.w3.org/2002/07/owl#sameAs>) \
          FILTER NOT EXISTS { ?s ?p ?o2 } \
        }",

        // ── Property rules (OWL 2 RL Table 5) ────────────────────────────────────

        // Rule 23: prp-fp — FunctionalProperty: two objects of the same subject → sameAs
        "INSERT { ?y1 <http://www.w3.org/2002/07/owl#sameAs> ?y2 } \
        WHERE { \
          ?p <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#FunctionalProperty> . \
          ?x ?p ?y1 . \
          ?x ?p ?y2 . \
          FILTER(?y1 != ?y2) \
          FILTER NOT EXISTS { ?y1 <http://www.w3.org/2002/07/owl#sameAs> ?y2 } \
        }",

        // Rule 24: prp-ifp — InverseFunctionalProperty: two subjects for same object → sameAs
        "INSERT { ?x1 <http://www.w3.org/2002/07/owl#sameAs> ?x2 } \
        WHERE { \
          ?p <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#InverseFunctionalProperty> . \
          ?x1 ?p ?y . \
          ?x2 ?p ?y . \
          FILTER(?x1 != ?x2) \
          FILTER NOT EXISTS { ?x1 <http://www.w3.org/2002/07/owl#sameAs> ?x2 } \
        }",

        // Rule 25: prp-eqp1 — equivalentProperty: forward propagation of property assertions
        "INSERT { ?x ?p2 ?y } \
        WHERE { \
          ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 . \
          FILTER(!isBlank(?p2)) \
          ?x ?p1 ?y . \
          FILTER NOT EXISTS { ?x ?p2 ?y } \
        }",

        // Rule 26: prp-eqp2 — equivalentProperty: backward propagation of property assertions
        "INSERT { ?x ?p1 ?y } \
        WHERE { \
          ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 . \
          FILTER(!isBlank(?p1)) \
          ?x ?p2 ?y . \
          FILTER NOT EXISTS { ?x ?p1 ?y } \
        }",

        // Rule 27: prp-spo2 — propertyChainAxiom (length-2 chains only)
        //   For ?p owl:propertyChainAxiom (?p1 ?p2): ?u1 ?p1 ?u2, ?u2 ?p2 ?u3 → ?u1 ?p ?u3.
        "INSERT { ?u1 ?p ?u3 } \
        WHERE { \
          ?p <http://www.w3.org/2002/07/owl#propertyChainAxiom> ?chain . \
          ?chain <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?p1 ; \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>  ?tail . \
          ?tail  <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?p2 ; \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> \
                   <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> . \
          ?u1 ?p1 ?u2 . \
          ?u2 ?p2 ?u3 . \
          FILTER NOT EXISTS { ?u1 ?p ?u3 } \
        }",

        // ── Class rules (OWL 2 RL Table 6) ───────────────────────────────────────

        // Rule 28: cls-svf1 (raw) — someValuesFrom restriction → assert blank-node restriction type
        //   Needed so cls-int1 can fire for intersections that mix blank-node restrictions.
        "INSERT { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        WHERE { \
          ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> . \
          ?x <http://www.w3.org/2002/07/owl#someValuesFrom> ?y . \
          FILTER(!isBlank(?y)) \
          FILTER(?y != <http://www.w3.org/2002/07/owl#Thing>) \
          ?x <http://www.w3.org/2002/07/owl#onProperty> ?p . \
          ?u ?p ?v . \
          ?v <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?y . \
          FILTER NOT EXISTS { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        }",

        // Rule 29: cls-svf2 — someValuesFrom owl:Thing: any value for property satisfies restriction
        "INSERT { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#someValuesFrom> <http://www.w3.org/2002/07/owl#Thing> . \
          ?x <http://www.w3.org/2002/07/owl#onProperty> ?p . \
          ?u ?p ?v . \
          FILTER NOT EXISTS { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        }",

        // Rule 30: cls-hv1 — hasValue restriction → derive the required property assertion
        //   If ?u is of type a hasValue restriction, assert the property value for ?u.
        "INSERT { ?u ?p ?y } \
        WHERE { \
          ?x <http://www.w3.org/2002/07/owl#hasValue>   ?y . \
          ?x <http://www.w3.org/2002/07/owl#onProperty> ?p . \
          ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x . \
          FILTER NOT EXISTS { ?u ?p ?y } \
        }",

        // Rule 31: cls-hv2 (raw) — hasValue restriction → assert blank-node restriction type
        //   Needed so cls-int1 can fire for intersections that mix hasValue restrictions.
        "INSERT { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        WHERE { \
          ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
               <http://www.w3.org/2002/07/owl#Restriction> . \
          ?x <http://www.w3.org/2002/07/owl#hasValue>   ?y . \
          ?x <http://www.w3.org/2002/07/owl#onProperty> ?p . \
          ?u ?p ?y . \
          FILTER NOT EXISTS { ?u <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x } \
        }",

        // Rule 32: cls-oo — owl:oneOf: every enumerated individual is an instance of the class
        "INSERT { ?yi <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#oneOf> ?x . \
          ?x (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?yi . \
          FILTER NOT EXISTS { ?yi <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        }",

        // Rule 33: cls-int1 (general) — direct owl:intersectionOf on named class
        //   Fires when a named class directly carries owl:intersectionOf (rare in OWL/XML but
        //   valid) and the individual is typed as every member class.
        "INSERT { ?y <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          FILTER(!isBlank(?c)) \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m . \
            FILTER NOT EXISTS { ?y <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?m } \
          } \
          ?ilist <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?m1 . \
          ?y <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?m1 . \
          FILTER NOT EXISTS { ?y <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        }",

        // Rule 34: cls-int2 (general) — direct owl:intersectionOf member propagation on named class
        "INSERT { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?member } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          FILTER(!isBlank(?c)) \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?member . \
          FILTER(!isBlank(?member)) \
          ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c . \
          FILTER NOT EXISTS { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?member } \
        }",

        // Rule 35: cls-uni (general) — direct owl:unionOf on named class
        "INSERT { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#unionOf> ?ulist . \
          FILTER(!isBlank(?c)) \
          ?ulist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?member . \
          FILTER(!isBlank(?member)) \
          ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?member . \
          FILTER NOT EXISTS { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c } \
        }",

        // Rule 36: cls-int1 via equivalentClass (general — allows typed blank-node members)
        //   Supplement to Rule 5: fires when the equivalentClass blank-node intersection
        //   has some members that are blank-node restrictions (typed via Rules 28/31).
        "INSERT { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?C } \
        WHERE { \
          ?C <http://www.w3.org/2002/07/owl#equivalentClass> ?bn . \
          ?bn <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          FILTER NOT EXISTS { \
            ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?m . \
            FILTER NOT EXISTS { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?m } \
          } \
          ?ilist <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?m1 . \
          ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?m1 . \
          FILTER NOT EXISTS { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?C } \
        }",

        // ── Schema rules (OWL 2 RL Table 9) ──────────────────────────────────────

        // Rule 37: scm-sco — rdfs:subClassOf transitivity
        //   Materialises transitive closure so that scm-dom1/rng1 and other schema rules
        //   fire across multi-hop subclass chains without needing extra fixpoint iterations.
        "INSERT { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c3 } \
        WHERE { \
          ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 . \
          ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c3 . \
          FILTER(!isBlank(?c3)) \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c3 } \
        }",

        // Rule 38a: scm-eqc1 forward — equivalentClass → subClassOf
        "INSERT { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#equivalentClass> ?c2 . \
          FILTER(!isBlank(?c1)) FILTER(!isBlank(?c2)) \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        }",

        // Rule 38b: scm-eqc1 backward — equivalentClass → subClassOf (reverse direction)
        "INSERT { ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c1 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#equivalentClass> ?c2 . \
          FILTER(!isBlank(?c1)) FILTER(!isBlank(?c2)) \
          FILTER NOT EXISTS { ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c1 } \
        }",

        // Rule 39: scm-eqc2 — mutual rdfs:subClassOf → owl:equivalentClass
        "INSERT { ?c1 <http://www.w3.org/2002/07/owl#equivalentClass> ?c2 } \
        WHERE { \
          ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 . \
          ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c1 . \
          FILTER(!isBlank(?c1)) FILTER(!isBlank(?c2)) \
          FILTER(?c1 != ?c2) \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2002/07/owl#equivalentClass> ?c2 } \
        }",

        // Rule 40: scm-spo — rdfs:subPropertyOf transitivity
        "INSERT { ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p3 } \
        WHERE { \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          ?p2 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p3 . \
          FILTER(!isBlank(?p3)) \
          FILTER NOT EXISTS { ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p3 } \
        }",

        // Rule 41a: scm-eqp1 forward — equivalentProperty → subPropertyOf
        "INSERT { ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 } \
        WHERE { \
          ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 . \
          FILTER(!isBlank(?p1)) FILTER(!isBlank(?p2)) \
          FILTER NOT EXISTS { ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 } \
        }",

        // Rule 41b: scm-eqp1 backward — equivalentProperty → subPropertyOf (reverse direction)
        "INSERT { ?p2 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p1 } \
        WHERE { \
          ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 . \
          FILTER(!isBlank(?p1)) FILTER(!isBlank(?p2)) \
          FILTER NOT EXISTS { ?p2 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p1 } \
        }",

        // Rule 42: scm-eqp2 — mutual rdfs:subPropertyOf → owl:equivalentProperty
        "INSERT { ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 } \
        WHERE { \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          ?p2 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p1 . \
          FILTER(!isBlank(?p1)) FILTER(!isBlank(?p2)) \
          FILTER(?p1 != ?p2) \
          FILTER NOT EXISTS { ?p1 <http://www.w3.org/2002/07/owl#equivalentProperty> ?p2 } \
        }",

        // Rule 43: scm-dom1 — rdfs:domain propagation through rdfs:subClassOf
        //   If P rdfs:domain C1 and C1 rdfs:subClassOf C2, infer P rdfs:domain C2.
        "INSERT { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c2 } \
        WHERE { \
          ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c1 . \
          ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 . \
          FILTER(!isBlank(?c2)) \
          FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c2 } \
        }",

        // Rule 44: scm-dom2 — rdfs:domain propagation through rdfs:subPropertyOf
        //   If P2 rdfs:domain C and P1 rdfs:subPropertyOf P2, infer P1 rdfs:domain C.
        "INSERT { ?p1 <http://www.w3.org/2000/01/rdf-schema#domain> ?c } \
        WHERE { \
          ?p2 <http://www.w3.org/2000/01/rdf-schema#domain> ?c . \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          FILTER NOT EXISTS { ?p1 <http://www.w3.org/2000/01/rdf-schema#domain> ?c } \
        }",

        // Rule 45: scm-rng1 — rdfs:range propagation through rdfs:subClassOf
        //   If P rdfs:range C1 and C1 rdfs:subClassOf C2, infer P rdfs:range C2.
        "INSERT { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c2 } \
        WHERE { \
          ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c1 . \
          ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 . \
          FILTER(!isBlank(?c2)) \
          FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c2 } \
        }",

        // Rule 46: scm-rng2 — rdfs:range propagation through rdfs:subPropertyOf
        //   If P2 rdfs:range C and P1 rdfs:subPropertyOf P2, infer P1 rdfs:range C.
        "INSERT { ?p1 <http://www.w3.org/2000/01/rdf-schema#range> ?c } \
        WHERE { \
          ?p2 <http://www.w3.org/2000/01/rdf-schema#range> ?c . \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          FILTER NOT EXISTS { ?p1 <http://www.w3.org/2000/01/rdf-schema#range> ?c } \
        }",

        // Rule 47: scm-int — intersectionOf class → rdfs:subClassOf each member
        //   Infers that the intersection class is a subclass of every component class.
        "INSERT { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?ci } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#intersectionOf> ?ilist . \
          FILTER(!isBlank(?c)) \
          ?ilist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?ci . \
          FILTER(!isBlank(?ci)) \
          FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?ci } \
        }",

        // Rule 48: scm-uni — each union member → rdfs:subClassOf the union class
        "INSERT { ?ci <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c } \
        WHERE { \
          ?c <http://www.w3.org/2002/07/owl#unionOf> ?ulist . \
          FILTER(!isBlank(?c)) \
          ?ulist (<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>*/<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>) ?ci . \
          FILTER(!isBlank(?ci)) \
          FILTER NOT EXISTS { ?ci <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c } \
        }",

        // Rule 49: scm-svf1 — someValuesFrom class subsumption
        //   If c1 = ∃p.y1, c2 = ∃p.y2, y1 ⊑ y2, then c1 ⊑ c2.
        "INSERT { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#someValuesFrom> ?y1 . \
          ?c1 <http://www.w3.org/2002/07/owl#onProperty>     ?p . \
          ?c2 <http://www.w3.org/2002/07/owl#someValuesFrom> ?y2 . \
          ?c2 <http://www.w3.org/2002/07/owl#onProperty>     ?p . \
          ?y1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?y2 . \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        }",

        // Rule 50: scm-svf2 — someValuesFrom property subsumption
        //   If c1 = ∃p1.y, c2 = ∃p2.y, p1 ⊑ p2, then c1 ⊑ c2.
        "INSERT { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#someValuesFrom> ?y . \
          ?c1 <http://www.w3.org/2002/07/owl#onProperty>     ?p1 . \
          ?c2 <http://www.w3.org/2002/07/owl#someValuesFrom> ?y . \
          ?c2 <http://www.w3.org/2002/07/owl#onProperty>     ?p2 . \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        }",

        // Rule 51: scm-avf1 — allValuesFrom class subsumption
        //   If c1 = ∀p.y1, c2 = ∀p.y2, y1 ⊑ y2, then c1 ⊑ c2.
        "INSERT { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#allValuesFrom> ?y1 . \
          ?c1 <http://www.w3.org/2002/07/owl#onProperty>    ?p . \
          ?c2 <http://www.w3.org/2002/07/owl#allValuesFrom> ?y2 . \
          ?c2 <http://www.w3.org/2002/07/owl#onProperty>    ?p . \
          ?y1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?y2 . \
          FILTER NOT EXISTS { ?c1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c2 } \
        }",

        // Rule 52: scm-avf2 — allValuesFrom property subsumption (REVERSED direction)
        //   If c1 = ∀p1.y, c2 = ∀p2.y, p1 ⊑ p2, then c2 ⊑ c1.
        //   (A stronger property constraint on a broader property range ⟹ more specific class.)
        "INSERT { ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c1 } \
        WHERE { \
          ?c1 <http://www.w3.org/2002/07/owl#allValuesFrom> ?y . \
          ?c1 <http://www.w3.org/2002/07/owl#onProperty>    ?p1 . \
          ?c2 <http://www.w3.org/2002/07/owl#allValuesFrom> ?y . \
          ?c2 <http://www.w3.org/2002/07/owl#onProperty>    ?p2 . \
          ?p1 <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p2 . \
          FILTER NOT EXISTS { ?c2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c1 } \
        }",
    ]
}
