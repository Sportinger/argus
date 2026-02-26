use std::collections::HashMap;
use std::sync::Arc;

use argus_core::Agent;

mod adsb;
mod ais;
mod eu_transparency;
mod gdelt;
mod opencorporates;
mod opensanctions;

pub use adsb::AdsbAgent;
pub use ais::AisAgent;
pub use eu_transparency::EuTransparencyAgent;
pub use gdelt::GdeltAgent;
pub use opencorporates::OpenCorporatesAgent;
pub use opensanctions::OpenSanctionsAgent;

pub fn agent_registry() -> HashMap<String, Arc<dyn Agent>> {
    let mut registry: HashMap<String, Arc<dyn Agent>> = HashMap::new();
    registry.insert("gdelt".into(), Arc::new(GdeltAgent::new()));
    registry.insert("opencorporates".into(), Arc::new(OpenCorporatesAgent::new()));
    registry.insert("ais".into(), Arc::new(AisAgent::new()));
    registry.insert("adsb".into(), Arc::new(AdsbAgent::new()));
    registry.insert("opensanctions".into(), Arc::new(OpenSanctionsAgent::new()));
    registry.insert(
        "eu_transparency".into(),
        Arc::new(EuTransparencyAgent::new()),
    );
    registry
}
