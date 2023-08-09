#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerModel {
    #[default]
    Steve,
    Alex,
}

impl PlayerModel {
    pub fn is_slim_arms(&self) -> bool {
        match self {
            PlayerModel::Steve => false,
            PlayerModel::Alex => true,
        }
    }
}
