// RGB standard library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use core::str::FromStr;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io;

use lnpbp::bitcoin::Txid;
use lnpbp::bp;
use lnpbp::bp::blind::OutpointHash;
use lnpbp::hex::FromHex;
use lnpbp::rgb::SealDefinition;
use lnpbp::strict_encoding::{self, StrictDecode, StrictEncode};

use super::AccountingValue;
use crate::error::ParseError;

#[derive(Clone, Debug, PartialEq, Display)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize,),
    serde(crate = "serde_crate")
)]
#[display(Debug)]
pub struct Outcoins {
    pub coins: AccountingValue,
    pub vout: u32,
    pub txid: Option<Txid>,
}

#[derive(Clone, Debug, PartialEq, Display)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize,),
    serde(crate = "serde_crate")
)]
#[display(Debug)]
pub struct Outcoincealed {
    pub coins: AccountingValue,
    pub seal_confidential: OutpointHash,
}

impl Outcoins {
    pub fn seal_definition(&self) -> SealDefinition {
        use lnpbp::bitcoin::secp256k1::rand::{self, RngCore};
        let mut rng = rand::thread_rng();
        let entropy = rng.next_u64(); // Not an amount blinding factor but outpoint blinding
        match self.txid {
            Some(txid) => {
                SealDefinition::TxOutpoint(bp::blind::OutpointReveal {
                    blinding: entropy,
                    txid,
                    vout: self.vout,
                })
            }
            None => SealDefinition::WitnessVout {
                vout: self.vout,
                blinding: entropy,
            },
        }
    }
}

impl StrictEncode for Outcoins {
    type Error = strict_encoding::Error;

    fn strict_encode<E: io::Write>(
        &self,
        mut e: E,
    ) -> Result<usize, Self::Error> {
        Ok(strict_encode_list!(e; self.coins, self.vout, self.txid))
    }
}

impl StrictDecode for Outcoins {
    type Error = strict_encoding::Error;

    fn strict_decode<D: io::Read>(mut d: D) -> Result<Self, Self::Error> {
        Ok(Self {
            coins: f32::strict_decode(&mut d)?,
            vout: u32::strict_decode(&mut d)?,
            txid: Option::<Txid>::strict_decode(&mut d)?,
        })
    }
}

impl StrictEncode for Outcoincealed {
    type Error = strict_encoding::Error;

    fn strict_encode<E: io::Write>(
        &self,
        mut e: E,
    ) -> Result<usize, Self::Error> {
        Ok(strict_encode_list!(e; self.coins, self.seal_confidential))
    }
}

impl StrictDecode for Outcoincealed {
    type Error = strict_encoding::Error;

    fn strict_decode<D: io::Read>(mut d: D) -> Result<Self, Self::Error> {
        Ok(Self {
            coins: f32::strict_decode(&mut d)?,
            seal_confidential: OutpointHash::strict_decode(&mut d)?,
        })
    }
}

impl FromStr for Outcoins {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(
            r"(?x)
                ^(?P<coins>[\d.,_']+) # float amount
                @
                ((?P<txid>[a-f\d]{64}) # Txid
                :)
                (?P<vout>\d+)$ # Vout
            ",
        )
        .expect("Regex parse failure");
        if let Some(m) = re.captures(&s.to_ascii_lowercase()) {
            match (m.name("coins"), m.name("txid"), m.name("vout")) {
                (Some(amount), Some(txid), Some(vout)) => Ok(Self {
                    coins: amount.as_str().parse()?,
                    vout: vout.as_str().parse()?,
                    txid: Some(Txid::from_hex(txid.as_str())?),
                }),
                (Some(amount), None, Some(vout)) => Ok(Self {
                    coins: amount.as_str().parse()?,
                    vout: vout.as_str().parse()?,
                    txid: None,
                }),
                _ => Err(ParseError),
            }
        } else {
            Err(ParseError)
        }
    }
}

impl FromStr for Outcoincealed {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(
            r"(?x)
                ^(?P<coins>[\d.,_']+) # float amount
                @
                ((?P<seal>[a-f\d]{64}))$ # Confidential seal: outpoint hash
            ",
        )
        .expect("Regex parse failure");
        if let Some(m) = re.captures(&s.to_ascii_lowercase()) {
            match (m.name("coins"), m.name("seal")) {
                (Some(amount), Some(seal)) => Ok(Self {
                    coins: amount.as_str().parse()?,
                    seal_confidential: OutpointHash::from_hex(seal.as_str())?,
                }),
                _ => Err(ParseError),
            }
        } else {
            Err(ParseError)
        }
    }
}
