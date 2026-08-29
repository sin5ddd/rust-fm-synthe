use serde::Deserialize;
use std::fmt;
use std::str::FromStr;

/// Yamaha 4-op style algorithms (TX81Z / DX21 family, 1–8).
///
/// Operators are numbered OP1..OP4. OP1 is the primary carrier in stacked
/// algorithms. Feedback is applied to one operator (default OP4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Algorithm {
    /// 4 → 3 → 2 → 1  (full stack)
    Serial = 1,
    /// (4 + 3) → 2 → 1
    ParallelMod = 2,
    /// 4 → 3 → 1  and  2 → 1
    DoubleMod = 3,
    /// 4 → 3 → 1  and  4 → 2 → 1
    SharedMod = 4,
    /// 4 → 3   +   2 → 1   (two 2-op stacks)
    DualStack = 5,
    /// 4 → 3, 4 → 2, 4 → 1
    TripleCarrier = 6,
    /// 4 → 3   +   2   +   1
    StackPlusCarriers = 7,
    /// 4 + 3 + 2 + 1  (additive, feedback on OP4)
    AllCarriers = 8,
}

impl Algorithm {
    pub const ALL: [Algorithm; 8] = [
        Algorithm::Serial,
        Algorithm::ParallelMod,
        Algorithm::DoubleMod,
        Algorithm::SharedMod,
        Algorithm::DualStack,
        Algorithm::TripleCarrier,
        Algorithm::StackPlusCarriers,
        Algorithm::AllCarriers,
    ];

    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            1 => Algorithm::Serial,
            2 => Algorithm::ParallelMod,
            3 => Algorithm::DoubleMod,
            4 => Algorithm::SharedMod,
            5 => Algorithm::DualStack,
            6 => Algorithm::TripleCarrier,
            7 => Algorithm::StackPlusCarriers,
            8 => Algorithm::AllCarriers,
            _ => return None,
        })
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Algorithm::Serial => "serial",
            Algorithm::ParallelMod => "parallel-mod",
            Algorithm::DoubleMod => "double-mod",
            Algorithm::SharedMod => "shared-mod",
            Algorithm::DualStack => "dual-stack",
            Algorithm::TripleCarrier => "triple-carrier",
            Algorithm::StackPlusCarriers => "stack-plus-carriers",
            Algorithm::AllCarriers => "all-carriers",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Algorithm::Serial => "4→3→2→1 直列。倍音が厚く、ベースやリード向き",
            Algorithm::ParallelMod => "(4+3)→2→1。モジュレータ並列で複雑な倍音",
            Algorithm::DoubleMod => "4→3→1 と 2→1。キャリア1本に2系統の変調",
            Algorithm::SharedMod => "4が3と2を駆動し、両方とも1へ。金属打楽器向き",
            Algorithm::DualStack => "4→3 と 2→1。2オペ2組。レイヤーやデチューン向き",
            Algorithm::TripleCarrier => "4が3/2/1を同時に変調。ベル・ヒット向き",
            Algorithm::StackPlusCarriers => "4→3 に素の2と1を加算。芯のあるスタブ",
            Algorithm::AllCarriers => "加算合成 + OP4フィードバック。オーガン/ノイズ寄り",
        }
    }

    /// How many operators mix to the audio output (carriers).
    pub fn carrier_count(self) -> usize {
        match self {
            Algorithm::Serial
            | Algorithm::ParallelMod
            | Algorithm::DoubleMod
            | Algorithm::SharedMod => 1,
            Algorithm::DualStack => 2,
            Algorithm::TripleCarrier | Algorithm::StackPlusCarriers => 3,
            Algorithm::AllCarriers => 4,
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.id(), self.name())
    }
}

impl FromStr for Algorithm {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let t = s.trim();
        if let Ok(id) = t.parse::<u8>() {
            return Algorithm::from_id(id)
                .ok_or_else(|| format!("algorithm id must be 1-8, got {id}"));
        }
        let key = t.to_ascii_lowercase().replace('_', "-");
        Algorithm::ALL
            .iter()
            .copied()
            .find(|a| a.name() == key)
            .ok_or_else(|| format!("unknown algorithm `{s}`"))
    }
}

impl<'de> Deserialize<'de> for Algorithm {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AlgoVisitor;
        impl<'de> serde::de::Visitor<'de> for AlgoVisitor {
            type Value = Algorithm;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "algorithm id 1-8 or name")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                if v > u64::from(u8::MAX) {
                    return Err(E::custom(format!("algorithm id {v} out of range")));
                }
                Algorithm::from_id(v as u8)
                    .ok_or_else(|| E::custom(format!("algorithm id must be 1-8, got {v}")))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                if v < 0 {
                    return Err(E::custom("algorithm id must be 1-8"));
                }
                self.visit_u64(v as u64)
            }

            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                v.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_any(AlgoVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_and_name() {
        assert_eq!("1".parse::<Algorithm>().unwrap(), Algorithm::Serial);
        assert_eq!(
            "dual-stack".parse::<Algorithm>().unwrap(),
            Algorithm::DualStack
        );
        assert_eq!("8".parse::<Algorithm>().unwrap(), Algorithm::AllCarriers);
    }
}
