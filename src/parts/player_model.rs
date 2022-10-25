#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerModel {
    Steve,
    Alex,
}

impl PlayerModel {
    pub fn get_dir_name(&self) -> &'static str {
        match self {
            PlayerModel::Steve => "Steve",
            PlayerModel::Alex => "Alex",
        }
    }
}
