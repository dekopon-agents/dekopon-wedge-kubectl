use std::fmt;
use std::io::Write;

use clap::Parser;
use dekopon_provider_sdk::provider::endpoint::Base;
use dekopon_provider_sdk::provider::{Http, Request, Settings, method};
use dekopon_provider_sdk::{
    Capability, Code, EffectKind, Failure, Proposal, Provider, RiskLevel, Stdout, Usage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(
    name = "kubectl",
    disable_help_flag = true,
    disable_help_subcommand = true
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(disable_help_flag = true)]
    Get {
        #[arg(long, value_parser = ["/readyz"])]
        raw: String,
    },
}

pub struct Kubectl;

impl Provider for Kubectl {
    const ID: &'static str = "kubectl";
    const COMMAND_WORDS: &'static [&'static str] = &["kubectl"];
    const DESCRIPTION: &'static str = "Narrow Kubernetes operations";

    type Args = Args;
    type Capabilities = (Readyz,);

    fn propose(args: Args, _stdin_piped: bool) -> Result<Proposal<Self>, Usage> {
        match args.command {
            Command::Get { .. } => Ok(Proposal::to::<Readyz>(ReadyzInput {})),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KubectlSettings {
    base_url: Base,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadyzInput {}

pub struct Readyz;

impl Capability for Readyz {
    type Provider = Kubectl;
    const NAME: &'static str = "get.readyz";
    const DESCRIPTION: &'static str = "Check Kubernetes API server readiness";
    const EFFECT: EffectKind = EffectKind::ReadOnly;
    const RISK: RiskLevel = RiskLevel::Low;
    type Input = ReadyzInput;
    type Needs = (Http, Settings<KubectlSettings>);
    type Error = ReadyzError;

    fn run(
        _input: ReadyzInput,
        (http, settings): Self::Needs,
        out: &mut Stdout,
    ) -> Result<(), ReadyzError> {
        let base = settings.into_inner().base_url;
        let authority = base
            .as_str()
            .strip_prefix("https://")
            .ok_or(ReadyzError::InvalidOrigin)?;
        if authority.contains(['/', '\\']) {
            return Err(ReadyzError::InvalidOrigin);
        }
        let uri = base
            .join("/readyz")
            .map_err(|_| ReadyzError::InvalidOrigin)?;
        let request = Request::new(method::GET, uri).map_err(|_| ReadyzError::InvalidRequest)?;
        let response = http.send(request).map_err(|_| ReadyzError::HttpFailed)?;
        if !(200..300).contains(&response.status) {
            return Err(ReadyzError::UnexpectedStatus(response.status));
        }
        if !matches!(response.body.as_slice(), b"ok" | b"ok\n") {
            return Err(ReadyzError::InvalidResponse);
        }
        out.write_all(b"ok\n")
            .map_err(|_| ReadyzError::OutputFailed)
    }
}

#[derive(Debug)]
pub enum ReadyzError {
    InvalidOrigin,
    InvalidRequest,
    HttpFailed,
    UnexpectedStatus(u16),
    InvalidResponse,
    OutputFailed,
}

impl fmt::Display for ReadyzError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrigin => formatter.write_str("baseUrl must be an HTTPS origin"),
            Self::InvalidRequest => formatter.write_str("could not build the readiness request"),
            Self::HttpFailed => formatter.write_str("readiness request failed"),
            Self::UnexpectedStatus(status) => write!(formatter, "readiness returned HTTP {status}"),
            Self::InvalidResponse => formatter.write_str("readiness response did not contain ok"),
            Self::OutputFailed => formatter.write_str("could not write readiness result"),
        }
    }
}

impl Failure for ReadyzError {
    fn code(&self) -> Code {
        match self {
            Self::InvalidOrigin => Code::INVALID_SETTINGS,
            Self::InvalidRequest => Code::new("invalid-request"),
            Self::HttpFailed => Code::new("http-failed"),
            Self::UnexpectedStatus(_) => Code::new("unexpected-status"),
            Self::InvalidResponse => Code::new("invalid-response"),
            Self::OutputFailed => Code::new("output-failed"),
        }
    }
}

dekopon_provider_sdk::export!(Kubectl);
