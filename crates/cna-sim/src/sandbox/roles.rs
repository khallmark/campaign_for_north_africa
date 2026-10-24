//! Fictional sandbox capabilities, deliberately separate from sourced catalog stats.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Land,
    Air,
    Sea,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    None,
    Supply,
    Repair,
    Command,
    Fortify,
}

pub struct Role {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: Domain,
    pub motorized: bool,
    pub cp: i32,
    pub range: i32,
    pub strength: i32,
    pub support: Support,
    pub description: &'static str,
}

// These demonstration values are not SPI characteristics or historical ratings.
pub const ROLES: &[Role] = &[
    Role { id: "crusader", name: "Crusader tank", domain: Domain::Land, motorized: true, cp: 5, range: 1, strength: 40, support: Support::None, description: "Mobile armour. Captures objectives and fights adjacent ground units." },
    Role { id: "panzer", name: "Panzer tank", domain: Domain::Land, motorized: true, cp: 5, range: 1, strength: 40, support: Support::None, description: "Mobile armour. Available to either sandbox side." },
    Role { id: "infantry", name: "Infantry", domain: Domain::Land, motorized: false, cp: 3, range: 1, strength: 27, support: Support::None, description: "Foot troops. No fuel use; captures objectives and digs in." },
    Role { id: "truck", name: "Supply truck", domain: Domain::Land, motorized: true, cp: 5, range: 0, strength: 0, support: Support::Supply, description: "Transfers stores to friendly units within two hexes. Cannot attack." },
    Role { id: "italian-tank", name: "Italian tank", domain: Domain::Land, motorized: true, cp: 4, range: 1, strength: 34, support: Support::None, description: "Ground armour with a shorter movement allowance." },
    Role { id: "self-propelled-artillery", name: "Self-propelled artillery", domain: Domain::Land, motorized: true, cp: 4, range: 3, strength: 32, support: Support::None, description: "Mobile bombardment against ground targets up to three hexes away." },
    Role { id: "armored-car", name: "Armored car", domain: Domain::Land, motorized: true, cp: 6, range: 1, strength: 25, support: Support::None, description: "Fast armed ground reconnaissance." },
    Role { id: "motorcycle", name: "Motorcycle reconnaissance", domain: Domain::Land, motorized: true, cp: 6, range: 1, strength: 18, support: Support::None, description: "Fast light reconnaissance; avoids expensive ridge crossings." },
    Role { id: "field-artillery", name: "Field artillery", domain: Domain::Land, motorized: true, cp: 2, range: 3, strength: 35, support: Support::None, description: "Slow towed guns with three-hex ground bombardment." },
    Role { id: "anti-tank", name: "Anti-tank guns", domain: Domain::Land, motorized: true, cp: 2, range: 2, strength: 30, support: Support::None, description: "Two-hex ground fire; ten bonus damage against armour." },
    Role { id: "heavy-anti-air", name: "Heavy anti-aircraft guns", domain: Domain::Land, motorized: true, cp: 2, range: 2, strength: 30, support: Support::None, description: "Engages aircraft or ground targets within two hexes." },
    Role { id: "machinegun", name: "Machine-gun team", domain: Domain::Land, motorized: false, cp: 3, range: 1, strength: 30, support: Support::None, description: "Foot support weapons; fights adjacent ground targets without fuel." },
    Role { id: "headquarters", name: "Headquarters", domain: Domain::Land, motorized: false, cp: 3, range: 0, strength: 0, support: Support::Command, description: "Spends two CP to restore one CP to an adjacent friendly unit." },
    Role { id: "engineer", name: "Engineers", domain: Domain::Land, motorized: false, cp: 3, range: 1, strength: 22, support: Support::Fortify, description: "Spends one CP to fortify an adjacent ground combat unit." },
    Role { id: "tank-recovery", name: "Tank recovery vehicle", domain: Domain::Land, motorized: true, cp: 4, range: 0, strength: 0, support: Support::Repair, description: "Spends one CP and twenty fuel to repair twenty strength on an adjacent surviving ground vehicle." },
    Role { id: "camel-recon", name: "Camel reconnaissance", domain: Domain::Land, motorized: false, cp: 4, range: 1, strength: 18, support: Support::None, description: "Fuel-free desert reconnaissance." },
    Role { id: "fighter", name: "Fighter", domain: Domain::Air, motorized: true, cp: 8, range: 2, strength: 38, support: Support::None, description: "Flies over sea, land and enemy control; attacks aircraft only." },
    Role { id: "bomber", name: "Bomber", domain: Domain::Air, motorized: true, cp: 6, range: 2, strength: 40, support: Support::None, description: "Bombs ground or naval units; cannot capture or dig in." },
    Role { id: "warship", name: "Warship", domain: Domain::Sea, motorized: true, cp: 4, range: 3, strength: 38, support: Support::None, description: "Sea-only movement; naval and shore bombardment within three hexes." },
    Role { id: "ground-support", name: "Ground support unit", domain: Domain::Land, motorized: true, cp: 3, range: 0, strength: 0, support: Support::Supply, description: "Stocks and transfers fuel, water and ammunition within two hexes, including to aircraft." },
    Role { id: "biplane-fighter", name: "Biplane fighter", domain: Domain::Air, motorized: true, cp: 6, range: 1, strength: 28, support: Support::None, description: "Short-range air interception; ignores ground terrain and control." },
    Role { id: "flying-boat", name: "Flying boat", domain: Domain::Air, motorized: true, cp: 6, range: 2, strength: 30, support: Support::None, description: "Flies over both land and sea; attacks naval targets." },
    Role { id: "coastal-ship", name: "Coastal vessel", domain: Domain::Sea, motorized: true, cp: 5, range: 2, strength: 25, support: Support::None, description: "Sea-only coastal patrol; fires at ships and shore units." },
    Role { id: "desert-raider", name: "Desert raiders", domain: Domain::Land, motorized: true, cp: 6, range: 1, strength: 25, support: Support::None, description: "Fast ground raiders. Captures objectives with light firepower." },
];

pub fn role(id: &str) -> Option<&'static Role> {
    ROLES.iter().find(|role| role.id == id)
}

impl Role {
    pub fn can_target(&self, target: &Role) -> bool {
        self.range > 0
            && match self.id {
                "fighter" | "biplane-fighter" => target.domain == Domain::Air,
                "flying-boat" => target.domain == Domain::Sea,
                "heavy-anti-air" => target.domain != Domain::Sea,
                "bomber" | "warship" | "coastal-ship" => target.domain != Domain::Air,
                _ => target.domain == Domain::Land,
            }
    }

    pub fn can_capture(&self) -> bool {
        self.domain == Domain::Land && self.strength > 0
    }

    pub fn domain_name(&self) -> &'static str {
        match self.domain {
            Domain::Land => "Land",
            Domain::Air => "Air",
            Domain::Sea => "Sea",
        }
    }
}
