pub enum PlayerModel {
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
