use super::part::{MinecraftPosition, PartAnchorInfo};

#[cfg(feature = "markers")]
#[derive(Debug, Clone)]
pub struct Marker {
    pub name: String,
    pub position: MinecraftPosition,
}

#[cfg(feature = "markers")]
impl Marker {
    pub fn new(name: String, position: MinecraftPosition) -> Self {
        Self { name, position }
    }
}

#[derive(Debug, Clone)]
pub struct PartTrackingData {
    #[cfg(feature = "part_tracker")]
    name: Option<String>,
    #[cfg(feature = "part_tracker")]
    last_rotation: Option<(MinecraftPosition, PartAnchorInfo)>,
    #[cfg(feature = "part_tracker")]
    group: Vec<String>,
    #[cfg(feature = "markers")]
    markers: Vec<Marker>,
}

impl PartTrackingData {
    pub fn new(
        name: Option<String>,
    ) -> Self {
        Self {
            name,
            last_rotation: None,
            group: Vec::new(),
            #[cfg(feature = "markers")]
            markers: Vec::new(),
        }
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
    
    pub fn name_mut(&mut self) -> &mut Option<String> {
        &mut self.name
    }

    pub fn last_rotation(&self) -> Option<&(MinecraftPosition, PartAnchorInfo)> {
        self.last_rotation.as_ref()
    }
    
    pub fn last_rotation_mut(&mut self) -> &mut Option<(MinecraftPosition, PartAnchorInfo)> {
        &mut self.last_rotation
    }

    pub fn group(&self) -> &[String] {
        &self.group
    }
    
    pub fn push_group(&mut self, group: String) {
        self.group.push(group);
    }
    
    pub fn push_groups(&mut self, groups: Vec<String>) {
        self.group.extend(groups);
    }

    #[cfg(feature = "markers")]
    pub fn push_marker(&mut self, marker: Marker) {
        self.markers.push(marker);
    }
    
    #[cfg(feature = "markers")]
    pub fn push_markers(&mut self, markers: Vec<Marker>) {
        self.markers.extend(markers);
    }
    
    #[cfg(feature = "markers")]
    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }
}
