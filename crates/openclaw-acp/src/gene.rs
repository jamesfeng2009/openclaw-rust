//! ACP Gene/Capsule System
//!
//! Inspired by EvoMap/GEP, provides genetic encoding for agent capabilities.

use serde::{Deserialize, Serialize};

/// Gene - basic unit of agent capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub gene_id: String,
    pub gene_type: GeneType,
    pub expression: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GeneType {
    Input,
    Output,
    Control,
    Binding,
    Link,
    Rnc,
}

impl Gene {
    pub fn new(gene_type: GeneType, expression: String) -> Self {
        Self {
            gene_id: uuid::Uuid::new_v4().to_string(),
            gene_type,
            expression,
            weight: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Gene Capsule - collection of genes forming a complete agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneCapsule {
    pub capsule_id: String,
    pub name: String,
    pub description: String,
    pub genes: Vec<Gene>,
    pub version: String,
}

impl GeneCapsule {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            capsule_id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            genes: Vec::new(),
            version: "1.0.0".to_string(),
        }
    }

    pub fn add_gene(&mut self, gene: Gene) {
        self.genes.push(gene);
    }

    pub fn get_genes_by_type(&self, gene_type: &GeneType) -> Vec<&Gene> {
        self.genes.iter().filter(|g| &g.gene_type == gene_type).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gene_creation() {
        let gene = Gene::new(GeneType::Input, "query".to_string());
        assert_eq!(gene.gene_type, GeneType::Input);
    }

    #[test]
    fn test_capsule() {
        let mut capsule = GeneCapsule::new("test", "Test capsule");
        capsule.add_gene(Gene::new(GeneType::Input, "input1".to_string()));
        capsule.add_gene(Gene::new(GeneType::Output, "output1".to_string()));
        
        assert_eq!(capsule.genes.len(), 2);
    }
}
