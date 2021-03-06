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

use std::sync::Arc;

use lnpbp::bitcoin::OutPoint;
use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::transport::zmqsocket::ZmqType;
use lnpbp::lnp::{
    session, transport, CreateUnmarshaller, PlainTranscoder, Session,
    Unmarshall, Unmarshaller,
};
use lnpbp::rgb::{Consignment, ContractId, Genesis, SchemaId};

use super::{Config, Error};
use crate::api::fungible::{self, AcceptApi, Issue, TransferApi};
use crate::api::stash;
use crate::api::Reply;
use crate::cli::OutputFormat;
use crate::error::{BootstrapError, ServiceErrorDomain};
use crate::DataFormat;

pub struct Runtime {
    stash_rpc: session::Raw<PlainTranscoder, transport::zmqsocket::Connection>,
    fungible_rpc:
        session::Raw<PlainTranscoder, transport::zmqsocket::Connection>,
    unmarshaller: Unmarshaller<Reply>,
}

impl Runtime {
    pub async fn init(config: Config) -> Result<Self, BootstrapError> {
        let fungible_rpc = session::Raw::with_zmq_unencrypted(
            ZmqType::Req,
            &config.fungible_endpoint,
            None,
            None,
        )?;
        let stash_rpc = session::Raw::with_zmq_unencrypted(
            ZmqType::Req,
            &config.stash_endpoint,
            None,
            None,
        )?;
        Ok(Self {
            stash_rpc,
            fungible_rpc,
            unmarshaller: Reply::create_unmarshaller(),
        })
    }

    fn stash_command(
        &mut self,
        command: stash::Request,
    ) -> Result<Arc<Reply>, ServiceErrorDomain> {
        let data = command.encode()?;
        self.stash_rpc.send_raw_message(&data)?;
        let raw = self.stash_rpc.recv_raw_message()?;
        let reply = self.unmarshaller.unmarshall(&raw)?;
        Ok(reply)
    }

    fn fungible_command(
        &mut self,
        command: fungible::Request,
    ) -> Result<Arc<Reply>, ServiceErrorDomain> {
        let data = command.encode()?;
        self.fungible_rpc.send_raw_message(&data)?;
        let raw = self.fungible_rpc.recv_raw_message()?;
        let reply = self.unmarshaller.unmarshall(&raw)?;
        Ok(reply)
    }

    #[inline]
    pub fn list_schemata(&mut self) -> Result<Arc<Reply>, Error> {
        Ok(self.stash_command(stash::Request::ListSchemata())?)
    }

    #[inline]
    pub fn list_geneses(&mut self) -> Result<Arc<Reply>, Error> {
        Ok(self.stash_command(stash::Request::ListGeneses())?)
    }

    #[inline]
    pub fn schema(&mut self, schema_id: SchemaId) -> Result<Arc<Reply>, Error> {
        Ok(self.stash_command(stash::Request::ReadSchema(schema_id))?)
    }

    #[inline]
    pub fn genesis(
        &mut self,
        contract_id: ContractId,
    ) -> Result<Arc<Reply>, Error> {
        Ok(self.stash_command(stash::Request::ReadGenesis(contract_id))?)
    }

    #[inline]
    pub fn list(
        &mut self,
        output_format: OutputFormat,
    ) -> Result<Arc<Reply>, Error> {
        let data_format = match output_format {
            OutputFormat::Yaml => DataFormat::Yaml,
            OutputFormat::Json => DataFormat::Json,
            OutputFormat::Toml => DataFormat::Toml,
            OutputFormat::StrictEncode => DataFormat::StrictEncode,
            _ => unimplemented!("The provided output format is not supported for this operation")
        };
        Ok(self.fungible_command(fungible::Request::Sync(data_format))?)
    }

    #[inline]
    pub fn import(&mut self, genesis: Genesis) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::ImportAsset(genesis))?)
    }

    #[inline]
    pub fn export(
        &mut self,
        asset_id: ContractId,
    ) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::ExportAsset(asset_id))?)
    }

    #[inline]
    pub fn issue(&mut self, issue: Issue) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::Issue(issue))?)
    }

    #[inline]
    pub fn transfer(
        &mut self,
        transfer: TransferApi,
    ) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::Transfer(transfer))?)
    }

    #[inline]
    pub fn validate(
        &mut self,
        consignment: Consignment,
    ) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::Validate(consignment))?)
    }

    #[inline]
    pub fn accept(&mut self, accept: AcceptApi) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::Accept(accept))?)
    }

    #[inline]
    pub fn forget(&mut self, outpoint: OutPoint) -> Result<Arc<Reply>, Error> {
        Ok(self.fungible_command(fungible::Request::Forget(outpoint))?)
    }
}
