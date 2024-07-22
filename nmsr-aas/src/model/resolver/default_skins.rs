use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, EnumCount, FromRepr, EnumIter, Serialize,
)]
pub enum DefaultSkin {
    #[default]
    Steve,
    Alex,
    Zuri,
    Sunny,
    Noor,
    Makena,
    Kai,
    Efe,
    Ari,
}

pub struct DefaultSkinResolver;

impl DefaultSkinResolver {
    pub fn is_default_skin_for_uuid_slim(uuid: Uuid) -> bool {
        java_floor_mod(java_uuid_hash(uuid), DefaultSkin::COUNT as i32 * 2)
            < DefaultSkin::COUNT as i32
    }

    pub fn get_default_skin_for_uuid(uuid: Uuid) -> DefaultSkin {
        let skin_idx = java_floor_mod(java_uuid_hash(uuid), DefaultSkin::COUNT as i32);

        DefaultSkin::from_repr(skin_idx as usize).unwrap_or_default()
    }

    pub fn resolve_default_skin_for_uuid_parts(
        uuid: Uuid,
        slim: Option<bool>,
    ) -> (DefaultSkin, bool) {
        (
            Self::get_default_skin_for_uuid(uuid),
            slim.unwrap_or_else(|| Self::is_default_skin_for_uuid_slim(uuid)),
        )
    }
    pub fn resolve_default_skin_for_uuid(uuid: Uuid, slim: Option<bool>) -> &'static str {
        let (default_skin, is_slim) = Self::resolve_default_skin_for_uuid_parts(uuid, slim);
        
        Self::resolve_default_skin(default_skin, is_slim)
    }

    pub fn resolve_default_skin(skin_name: DefaultSkin, is_slim: bool) -> &'static str {
        match (skin_name, is_slim) {
            (DefaultSkin::Alex, false) => {
                "1abc803022d8300ab7578b189294cce39622d9a404cdc00d3feacfdf45be6981"
            }
            (DefaultSkin::Alex, true) => {
                "46acd06e8483b176e8ea39fc12fe105eb3a2a4970f5100057e9d84d4b60bdfa7"
            }

            (DefaultSkin::Ari, false) => {
                "4c05ab9e07b3505dc3ec11370c3bdce5570ad2fb2b562e9b9dd9cf271f81aa44"
            }
            (DefaultSkin::Ari, true) => {
                "6ac6ca262d67bcfb3dbc924ba8215a18195497c780058a5749de674217721892"
            }

            (DefaultSkin::Efe, false) => {
                "daf3d88ccb38f11f74814e92053d92f7728ddb1a7955652a60e30cb27ae6659f"
            }
            (DefaultSkin::Efe, true) => {
                "fece7017b1bb13926d1158864b283b8b930271f80a90482f174cca6a17e88236"
            }

            (DefaultSkin::Kai, false) => {
                "e5cdc3243b2153ab28a159861be643a4fc1e3c17d291cdd3e57a7f370ad676f3"
            }
            (DefaultSkin::Kai, true) => {
                "226c617fde5b1ba569aa08bd2cb6fd84c93337532a872b3eb7bf66bdd5b395f8"
            }

            (DefaultSkin::Makena, false) => {
                "dc0fcfaf2aa040a83dc0de4e56058d1bbb2ea40157501f3e7d15dc245e493095"
            }
            (DefaultSkin::Makena, true) => {
                "7cb3ba52ddd5cc82c0b050c3f920f87da36add80165846f479079663805433db"
            }

            (DefaultSkin::Noor, false) => {
                "90e75cd429ba6331cd210b9bd19399527ee3bab467b5a9f61cb8a27b177f6789"
            }
            (DefaultSkin::Noor, true) => {
                "6c160fbd16adbc4bff2409e70180d911002aebcfa811eb6ec3d1040761aea6dd"
            }

            (DefaultSkin::Steve, false) => {
                "31f477eb1a7beee631c2ca64d06f8f68fa93a3386d04452ab27f43acdf1b60cb"
            }
            (DefaultSkin::Steve, true) => {
                "d5c4ee5ce20aed9e33e866c66caa37178606234b3721084bf01d13320fb2eb3f"
            }

            (DefaultSkin::Sunny, false) => {
                "a3bd16079f764cd541e072e888fe43885e711f98658323db0f9a6045da91ee7a"
            }
            (DefaultSkin::Sunny, true) => {
                "b66bc80f002b10371e2fa23de6f230dd5e2f3affc2e15786f65bc9be4c6eb71a"
            }

            (DefaultSkin::Zuri, false) => {
                "f5dddb41dcafef616e959c2817808e0be741c89ffbfed39134a13e75b811863d"
            }
            (DefaultSkin::Zuri, true) => {
                "eee522611005acf256dbd152e992c60c0bb7978cb0f3127807700e478ad97664"
            }
        }
    }
}

fn java_uuid_hash(uuid: Uuid) -> i32 {
    let (most_sig, least_sig) = uuid.as_u64_pair();
    let hilo = most_sig ^ least_sig;
    ((hilo >> 32) as i32) ^ (hilo as i32)
}

fn java_floor_mod(a: i32, b: i32) -> i32 {
    (a % b + b) % b
}

#[cfg(test)]
mod test {
    use uuid::uuid;

    use crate::model::resolver::default_skins::{java_floor_mod, java_uuid_hash};

    #[test]
    fn java_uuid_hash_test() {
        assert_eq!(
            java_uuid_hash(uuid!("8f59cdb2-f575-4768-b0e6-2e0b0be19557")),
            -1054133882
        );
        assert_eq!(
            java_uuid_hash(uuid!("0d8d9d75-4a5a-4579-9e3c-1063c2d9c505")),
            456265066
        );
        assert_eq!(
            java_uuid_hash(uuid!("09bb112f-1c6a-40db-a56b-337e61bcf387")),
            -788098803
        );
        assert_eq!(
            java_uuid_hash(uuid!("59a47393-2d6b-45bc-a88b-5eb8a731648c")),
            2071268379
        );
        assert_eq!(
            java_uuid_hash(uuid!("5c0a5e11-31ca-4b8e-9370-690ab1b94365")),
            1326006256
        );
        assert_eq!(
            java_uuid_hash(uuid!("86f601c6-6442-41cd-bbc7-fc267716ace4")),
            778375369
        );
        assert_eq!(
            java_uuid_hash(uuid!("9ad2f07a-5c5b-4ec8-9d0f-32abfa06da62")),
            -1585424773
        );
        assert_eq!(
            java_uuid_hash(uuid!("4a6f47d6-06eb-4130-9270-4edcf0e2459c")),
            773197222
        );
        assert_eq!(
            java_uuid_hash(uuid!("5b1c257e-fcf8-44dd-99a3-cc1e572cdd37")),
            1768648842
        );
        assert_eq!(
            java_uuid_hash(uuid!("d58a9d9a-e9cc-430e-8c9a-b8d4089115d5")),
            -1202883691
        );
        assert_eq!(
            java_uuid_hash(uuid!("c425728f-49fb-40f5-96e5-d2825385117b")),
            1220473219
        );
        assert_eq!(
            java_uuid_hash(uuid!("186b098b-3b9f-48ec-aa3d-319b74de83a8")),
            -48762028
        );
        assert_eq!(
            java_uuid_hash(uuid!("fccf762e-67b7-4a1b-b55d-505952ecb438")),
            2093602900
        );
        assert_eq!(
            java_uuid_hash(uuid!("c3196bef-f4fa-4913-8909-59c36e5e843c")),
            -793444605
        );
        assert_eq!(
            java_uuid_hash(uuid!("22300ca0-d770-44bb-b3c8-91edb148c64c")),
            -138403910
        );
        assert_eq!(
            java_uuid_hash(uuid!("4e6570fd-ec1d-44f3-88ee-c7f036ac4edc")),
            473611554
        );
        assert_eq!(
            java_uuid_hash(uuid!("dc8e6e4c-4230-4ce2-84e6-7753d051b7ef")),
            -905321966
        );
        assert_eq!(
            java_uuid_hash(uuid!("c3d6c522-99a5-48d4-9be3-76ae2c4b9d4b")),
            -304388589
        );
        assert_eq!(
            java_uuid_hash(uuid!("c1fee5b2-90a2-4b00-bffe-a7a5da9f9e27")),
            876451632
        );
        assert_eq!(
            java_uuid_hash(uuid!("72e38a51-8025-4141-92fc-21550c8e2db4")),
            1823786993
        );
    }

    #[test]
    fn floor_mod_test() {
        // https://mkyong.com/java/java-mod-examples/
        assert_eq!(java_floor_mod(5, -3), -1, "5 % -3 = -1");
        assert_eq!(java_floor_mod(-5, -3), -2, "-5 % -3 = -2");
        assert_eq!(java_floor_mod(-5, 3), 1, "-5 % 3 = 1");
        assert_eq!(java_floor_mod(-5, -3), -2, "-5 % -3 = -2");
        assert_eq!(java_floor_mod(-4, 3), 2, "-4 % 3 = 2");
        assert_eq!(java_floor_mod(-4, -3), -1, "-4 % -3 = -1");
        assert_eq!(java_floor_mod(-3, 3), 0, "-3 % 3 = 0");
        assert_eq!(java_floor_mod(-3, -3), 0, "-3 % -3 = 0");
        assert_eq!(java_floor_mod(-2, 3), 1, "-2 % 3 = 1");
        assert_eq!(java_floor_mod(-2, -3), -2, "-2 % -3 = -2");
        assert_eq!(java_floor_mod(-1, 3), 2, "-1 % 3 = 2");
        assert_eq!(java_floor_mod(-1, -3), -1, "-1 % -3 = -1");
        assert_eq!(java_floor_mod(0, 3), 0, "0 % 3 = 0");
        assert_eq!(java_floor_mod(0, -3), 0, "0 % -3 = 0");
        assert_eq!(java_floor_mod(1, 3), 1, "1 % 3 = 1");
        assert_eq!(java_floor_mod(1, -3), -2, "1 % -3 = -2");
        assert_eq!(java_floor_mod(2, 3), 2, "2 % 3 = 2");
        assert_eq!(java_floor_mod(2, -3), -1, "2 % -3 = -1");
        assert_eq!(java_floor_mod(3, 3), 0, "3 % 3 = 0");
        assert_eq!(java_floor_mod(3, -3), 0, "3 % -3 = 0");
        assert_eq!(java_floor_mod(4, 3), 1, "4 % 3 = 1");
        assert_eq!(java_floor_mod(4, -3), -2, "4 % -3 = -2");
        assert_eq!(java_floor_mod(5, 3), 2, "5 % 3 = 2");
        assert_eq!(java_floor_mod(5, -3), -1, "5 % -3 = -1");
    }
}
