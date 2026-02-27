use openclaw_acp::gene::{Gene, GeneCapsule, GeneType};
use crate::evo::registry::DynamicSkill;

pub struct EvoToAcpAdapter;

impl EvoToAcpAdapter {
    pub fn skill_to_gene(skill: &DynamicSkill) -> Gene {
        Gene {
            gene_id: skill.id.clone(),
            gene_type: GeneType::Control,
            expression: skill.code.clone(),
            weight: 1.0,
        }
    }

    pub fn skills_to_capsule(skills: Vec<DynamicSkill>, capsule_name: &str) -> GeneCapsule {
        let mut capsule = GeneCapsule::new(capsule_name, "Evo generated capability");
        for skill in skills {
            capsule.add_gene(Self::skill_to_gene(&skill));
        }
        capsule
    }

    pub fn skill_to_capsule(skill: DynamicSkill) -> GeneCapsule {
        let gene = Self::skill_to_gene(&skill);
        
        let mut capsule = GeneCapsule::new(
            format!("EvoSkill_{}", skill.name),
            format!("Auto-generated skill: {}", skill.name),
        );
        capsule.add_gene(gene);
        capsule
    }
}
