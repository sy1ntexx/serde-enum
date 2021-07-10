use serde_enum::{Deserialize_enum, Serialize_enum};

#[derive(Serialize_enum, Deserialize_enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Something {
    #[serde(name = "HiddenNotFormatted")]
    SomeVariant,
    OtherVariant,
}

// impl serde_enum::serde::Serialize for Something {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: ::serde_enum::serde::Serializer
//     {
//         match self {
//             Self::SomeVariant => serializer.serialize_str("someVariant"),
//             Self::OtherVariant => serializer.serialize_str("OtherVariant"),
//         }
//     }
// }
//
// impl<'de> serde_enum::serde::Deserialize<'de> for Something {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>
//     {
//         Ok(
//             match <&str>::deserialize(deserializer)? {
//                 "someVariant" => Self::SomeVariant,
//                 "OtherVariant" => Self::OtherVariant,
//                 _ => { unimplemented!() }
//             }
//         )
//     }
// }

fn main() {}
