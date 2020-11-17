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

use ::std::sync::Arc;
use std::fs::File;
use std::path::PathBuf;

use lnpbp::bitcoin::consensus::encode::{deserialize, Encodable};
use lnpbp::bitcoin::util::psbt::PartiallySignedTransaction;
use lnpbp::bitcoin::OutPoint;

use lnpbp::bp;
use lnpbp::bp::psbt::ProprietaryKeyMap;
use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::{Session, Unmarshall};
use lnpbp::rgb::PSBT_OUT_PUBKEY;

use super::{Error, Runtime};
use crate::api::{
    fungible::Issue, fungible::Request, fungible::TransferApi, reply, Reply,
};
use crate::error::ServiceErrorDomain;
use crate::fungible::{
    Invoice, IssueStructure, Outcoincealed, Outcoins, Outpoint,
};
use crate::util::file::ReadWrite;
use crate::util::SealSpec;
use crate::DataFormat;

impl Runtime {
    fn command(
        &mut self,
        command: Request,
    ) -> Result<Arc<Reply>, ServiceErrorDomain> {
        let data = command.encode()?;
        self.session_rpc.send_raw_message(&data)?;
        let raw = self.session_rpc.recv_raw_message()?;
        let reply = self.unmarshaller.unmarshall(&raw)?;
        Ok(reply)
    }

    pub fn issue(
        &mut self,
        _network: bp::Chain,
        ticker: String,
        title: String,
        description: Option<String>,
        issue_structure: IssueStructure,
        allocate: Vec<Outcoins>,
        precision: u8,
        _prune_seals: Vec<SealSpec>,
    ) -> Result<(), Error> {
        // TODO: Make sure we use the same network
        let (supply, inflatable) = match issue_structure {
            IssueStructure::SingleIssue => (None, None),
            IssueStructure::MultipleIssues {
                max_supply,
                reissue_control,
            } => (Some(max_supply), Some(reissue_control)),
        };
        let command = Request::Issue(Issue {
            ticker,
            title,
            description,
            supply,
            inflatable,
            precision,
            allocate,
        });
        match &*self.command(command)? {
            Reply::Success => Ok(()),
            Reply::Failure(failmsg) => Err(Error::Reply(failmsg.clone())),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    pub fn transfer(
        &mut self,
        inputs: Vec<OutPoint>,
        allocate: Vec<Outcoins>,
        invoice: Invoice,
        prototype_psbt: String,
        consignment_file: String,
        transaction_file: String,
    ) -> Result<(), Error> {
        let seal_confidential = match invoice.outpoint {
            Outpoint::BlindedUtxo(outpoint_hash) => outpoint_hash,
            Outpoint::Address(_address) => unimplemented!(),
        };

        let psbt_bytes = base64::decode(&prototype_psbt)?;
        let mut psbt: PartiallySignedTransaction = deserialize(&psbt_bytes)?;

        for (index, output) in &mut psbt.outputs.iter_mut().enumerate() {
            if let Some(key) = output.hd_keypaths.keys().next() {
                let key = key.clone();
                output.insert_proprietary_key(
                    b"RGB".to_vec(),
                    PSBT_OUT_PUBKEY,
                    vec![],
                    &key.to_bytes(),
                );
                debug!("Output #{} commitment key will be {}", index, key);
            } else {
                warn!(
                    "No public key information found for output #{}; \
                    LNPBP1/2 commitment will be impossible.\
                    In order to allow commitment pls add known keys derivation \
                    information to PSBT output map",
                    index
                );
            }
        }
        trace!("{:?}", psbt);

        let api = TransferApi {
            psbt,
            contract_id: invoice.contract_id,
            inputs,
            ours: allocate,
            theirs: vec![Outcoincealed {
                coins: invoice.amount,
                seal_confidential,
            }],
        };

        match &*self.command(Request::Transfer(api))? {
            Reply::Failure(failure) => Err(Error::Reply(failure.clone())),
            Reply::Transfer(transfer) => {
                transfer
                    .consignment
                    .write_file(PathBuf::from(&consignment_file))?;
                let out_file = File::create(&transaction_file)
                    .expect("can't create output transaction file");
                transfer.psbt.consensus_encode(out_file)?;
                println!(
                    "Transfer succeeded, consignment data are written to {:?}, partially signed witness transaction to {:?}",
                    consignment_file, transaction_file
                );

                Ok(())
            }
            _ => Err(Error::UnexpectedResponse),
        }
    }

    pub fn sync(
        &mut self,
        data_format: DataFormat,
    ) -> Result<reply::SyncFormat, Error> {
        match &*self.command(Request::Sync(data_format))? {
            Reply::Sync(data) => Ok(data.clone()),
            Reply::Failure(failmsg) => Err(Error::Reply(failmsg.clone())),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    pub fn outpoint_assets(
        &mut self,
        outpoint: OutPoint,
    ) -> Result<reply::AssetsFormat, Error> {
        match &*self.command(Request::Assets(outpoint))? {
            Reply::Assets(data) => Ok(reply::AssetsFormat(data.clone())),
            Reply::Failure(failmsg) => Err(Error::Reply(failmsg.clone())),
            _ => Err(Error::UnexpectedResponse),
        }
    }
}
