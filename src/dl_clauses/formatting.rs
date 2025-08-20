//! Output formatting methods for DL clauses
//! 
//! This module contains methods for formatting DL clauses into HermiT-style output,
//! including file saving and string conversion functionality.

use std::{fs::File, io::Write, path::Path};
use crate::error::Result;

use super::types::DLClauseSet;

impl DLClauseSet {
    /// Save DL clauses to a file in HermiT format
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path)?;
        
        // Write prefixes
        writeln!(file, "Prefixes: [")?;
        for (prefix, namespace) in &self.prefixes {
            if prefix.is_empty() {
                writeln!(file, "  : = <{namespace}>")?;
            } else {
                writeln!(file, "  {prefix}: = <{namespace}>")?;
            }
        }
        writeln!(file, "]")?;
        
        // Write deterministic DL clauses
        writeln!(file, "Deterministic DL-clauses: [")?;
        for clause in &self.deterministic_clauses {
            writeln!(file, "  {clause}")?;
        }
        writeln!(file, "]")?;
        
        // Write disjunctive DL clauses
        writeln!(file, "Disjunctive DL-clauses: [")?;
        for clause in &self.disjunctive_clauses {
            writeln!(file, "  {clause}")?;
        }
        writeln!(file, "]")?;
        
        // Write ABox facts
        writeln!(file, "ABox: [")?;
        for fact in &self.abox_facts {
            writeln!(file, "  {fact}")?;
        }
        writeln!(file, "]")?;
        
        // Write statistics
        writeln!(file, "Statistics: [")?;
        writeln!(file, "  Number of deterministic clauses: {}", self.statistics.deterministic_clause_count)?;
        writeln!(file, "  Number of nondeterministic clauses: {}", self.statistics.disjunctive_clause_count)?;
        writeln!(file, "  Number of disjunctions: {}", self.statistics.disjunction_count)?;
        writeln!(file, "  Number of positive facts: {}", self.statistics.positive_fact_count)?;
        writeln!(file, "  Number of negative facts: {}", self.statistics.negative_fact_count)?;
        writeln!(file, "]")?;
        
        Ok(())
    }

    /// Convert to HermiT-style string representation
    pub fn to_hermit_format(&self) -> String {
        let mut output = String::new();
        
        // Prefixes
        output.push_str("Prefixes: [\n");
        for (prefix, namespace) in &self.prefixes {
            if prefix.is_empty() {
                output.push_str(&format!("  : = <{namespace}>\n"));
            } else {
                output.push_str(&format!("  {prefix}: = <{namespace}>\n"));
            }
        }
        output.push_str("]\n");
        
        // Deterministic clauses
        output.push_str("Deterministic DL-clauses: [\n");
        for clause in &self.deterministic_clauses {
            output.push_str(&format!("  {clause}\n"));
        }
        output.push_str("]\n");
        
        // Disjunctive clauses
        output.push_str("Disjunctive DL-clauses: [\n");
        for clause in &self.disjunctive_clauses {
            output.push_str(&format!("  {clause}\n"));
        }
        output.push_str("]\n");
        
        // ABox
        output.push_str("ABox: [\n");
        for fact in &self.abox_facts {
            output.push_str(&format!("  {fact}\n"));
        }
        output.push_str("]\n");
        
        // Statistics
        output.push_str("Statistics: [\n");
        output.push_str(&format!("  Number of deterministic clauses: {}\n", self.statistics.deterministic_clause_count));
        output.push_str(&format!("  Number of nondeterministic clauses: {}\n", self.statistics.disjunctive_clause_count));
        output.push_str(&format!("  Number of disjunctions: {}\n", self.statistics.disjunction_count));
        output.push_str(&format!("  Number of positive facts: {}\n", self.statistics.positive_fact_count));
        output.push_str(&format!("  Number of negative facts: {}\n", self.statistics.negative_fact_count));
        output.push_str("]\n");
        
        output
    }

    /// Get a summary of the clause set
    pub fn summary(&self) -> String {
        format!(
            "DL Clause Set Summary:\n\
             - Deterministic clauses: {}\n\
             - Disjunctive clauses: {}\n\
             - ABox facts: {}\n\
             - Total disjunctions: {}",
            self.statistics.deterministic_clause_count,
            self.statistics.disjunctive_clause_count,
            self.abox_facts.len(),
            self.statistics.disjunction_count
        )
    }

    /// Get clauses in a compact format for debugging
    pub fn to_compact_format(&self) -> String {
        let mut output = String::new();
        
        output.push_str("=== Deterministic Clauses ===\n");
        for (i, clause) in self.deterministic_clauses.iter().enumerate() {
            output.push_str(&format!("{}: {}\n", i + 1, clause));
        }
        
        output.push_str("\n=== Disjunctive Clauses ===\n");
        for (i, clause) in self.disjunctive_clauses.iter().enumerate() {
            output.push_str(&format!("{}: {}\n", i + 1, clause));
        }
        
        output.push_str("\n=== ABox Facts ===\n");
        for (i, fact) in self.abox_facts.iter().enumerate() {
            output.push_str(&format!("{}: {}\n", i + 1, fact));
        }
        
        output
    }
}
