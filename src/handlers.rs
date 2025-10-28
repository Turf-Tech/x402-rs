//! HTTP endpoints implemented by the x402 **facilitator**.
//!
//! These are the server-side handlers for processing client-submitted x402 payments.
//! They include both protocol-critical endpoints (`/verify`, `/settle`) and discovery endpoints (`/supported`, etc).
//!
//! All payloads follow the types defined in the `x402-rs` crate, and are compatible
//! with the TypeScript and Go client SDKs.
//!
//! Each endpoint consumes or produces structured JSON payloads defined in `x402-rs`,
//! and is compatible with official x402 client SDKs.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router, response::IntoResponse};
use serde_json::json;
use tracing::instrument;

use crate::chain::FacilitatorLocalError;
use crate::facilitator::Facilitator;
use crate::types::{
    ErrorResponse, FacilitatorErrorReason, MixedAddress, SettleRequest, VerifyRequest,
    VerifyResponse,
};

/// `GET /verify`: Returns a machine-readable description of the `/verify` endpoint.
///
/// This is served by the facilitator to help clients understand how to construct
/// a valid [`VerifyRequest`] for payment verification.
///
/// This is optional metadata and primarily useful for discoverability and debugging tools.
#[instrument(skip_all)]
pub async fn get_verify_info() -> impl IntoResponse {
    Json(json!({
        "endpoint": "/verify",
        "description": "POST to verify x402 payments",
        "body": {
            "paymentPayload": "PaymentPayload",
            "paymentRequirements": "PaymentRequirements",
        }
    }))
}

/// `GET /settle`: Returns a machine-readable description of the `/settle` endpoint.
///
/// This is served by the facilitator to describe the structure of a valid
/// [`SettleRequest`] used to initiate on-chain payment settlement.
#[instrument(skip_all)]
pub async fn get_settle_info() -> impl IntoResponse {
    Json(json!({
        "endpoint": "/settle",
        "description": "POST to settle x402 payments",
        "body": {
            "paymentPayload": "PaymentPayload",
            "paymentRequirements": "PaymentRequirements",
        }
    }))
}

pub fn routes<A>() -> Router<A>
where
    A: Facilitator + Clone + Send + Sync + 'static,
    A::Error: IntoResponse,
{
    Router::new()
        .route("/", get(get_root))
        .route("/verify", get(get_verify_info))
        .route("/verify", post(post_verify::<A>))
        .route("/settle", get(get_settle_info))
        .route("/settle", post(post_settle::<A>))
        .route("/health", get(get_health::<A>))
        .route("/supported", get(get_supported::<A>))
}

/// `GET /`: Returns a terminal-style HTML landing page for the facilitator.
#[instrument(skip_all)]
pub async fn get_root() -> impl IntoResponse {
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");

    let html = format!(r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{pkg_name} — Avalanche Facilitator</title>
    <style>
      :root {{
        --bg: #0b1020;
        --panel: #0e1530;
        --text: #cfe3ff;
        --muted: #98a7c2;
        --accent: #e84142; /* Avalanche red */
        --green: #4ade80;
        --yellow: #fbbf24;
        --blue: #60a5fa;
        --cyan: #22d3ee;
        --shadow: 0 20px 40px rgba(0, 0, 0, 0.45);
      }}
      html, body {{ height: 100%; }}
      body {{
        margin: 0;
        background: radial-gradient(1200px 600px at 70% -10%, rgba(96,165,250,0.15), transparent 60%),
                    radial-gradient(900px 500px at -10% 20%, rgba(232,65,66,0.18), transparent 60%),
                    var(--bg);
        color: var(--text);
        font: 14px/1.6 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
        display: grid;
        place-items: center;
        padding: 32px;
      }}
      .wrap {{ width: 100%; max-width: 980px; }}
      .brand {{
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 16px;
        letter-spacing: .4px;
      }}
      .brand .left {{
        color: var(--muted);
      }}
      .brand .left strong {{ color: var(--text); }}
      .term {{
        background: linear-gradient(180deg, rgba(255,255,255,0.02), transparent 24px) , var(--panel);
        border: 1px solid rgba(255,255,255,0.06);
        border-radius: 10px;
        box-shadow: var(--shadow);
        overflow: hidden;
      }}
      .term-hdr {{
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 10px 12px;
        border-bottom: 1px solid rgba(255,255,255,0.06);
        background: rgba(255,255,255,0.02);
      }}
      .dot {{ width: 10px; height: 10px; border-radius: 50%; display: inline-block; }}
      .dot.red {{ background: #ff5f57; }}
      .dot.yellow {{ background: #febc2e; }}
      .dot.green {{ background: #28c840; }}
      .title {{ margin-left: 6px; color: var(--muted); }}
      pre {{ margin: 0; padding: 18px; white-space: pre-wrap; }}
      .prompt {{ color: var(--cyan); }}
      .cmd {{ color: var(--text); }}
      .ok {{ color: var(--green); }}
      .warn {{ color: var(--yellow); }}
      .accent {{ color: var(--accent); }}
      .dim {{ color: var(--muted); }}
      a {{ color: var(--blue); text-decoration: none; }}
      a:hover {{ text-decoration: underline; }}
      /* Tiny ASCII mascot, fixed to bottom-right, responsive */
      .art {{
        position: fixed;
        right: 8px;
        bottom: 8px;
        color: var(--muted);
        /* ~10x smaller than 11px → around 1.1px; use clamp for responsiveness */
        font-size: clamp(0.9px, 0.15vw, 1.4px);
        line-height: 1;
        white-space: pre;
        opacity: 0.6;
        pointer-events: none;
        user-select: none;
        padding: 0;
        margin: 0;
      }}
    </style>
  </head>
  <body>
    <div class="wrap">
      <div class="brand">
        <div class="left">⚡️ <strong>x402 Facilitator</strong> · Avalanche (EVM)</div>
        <div class="right dim">Powered by <strong>Turf Network</strong> 🧪</div>
      </div>

      <section class="term" aria-label="terminal">
        <div class="term-hdr">
          <span class="dot red"></span>
          <span class="dot yellow"></span>
          <span class="dot green"></span>
          <span class="title">{pkg_name} v{pkg_version}</span>
        </div>
        <pre>
<span class="prompt">user@x402-facilitator</span>:<span class="dim">~</span>$ <span class="cmd">./x402-rs --status</span>
✓ <span class="ok">Server</span> running on <span class="cmd">https://x402-avax.turf.network</span>
✓ <span class="ok">Health check</span>: PASSED

<span class="prompt">user@x402-facilitator</span>:<span class="dim">~</span>$ <span class="cmd">curl -s https://x402-avax.turf.network/supported | jq</span>
{{
  "networks": ["avalanche-fuji", "avalanche"],
  "scheme": "x402/erc-3009"
}}

<span class="prompt">user@x402-facilitator</span>:<span class="dim">~</span>$ <span class="cmd">./x402-rs --help</span>
GET  /            → terminal UI (this page)
GET  /health      → health check
GET  /supported   → supported networks & schemes
GET  /verify      → verification schema
POST /verify      → verify payment
GET  /settle      → settlement schema
POST /settle      → execute settlement

⛓️  <span class="accent">Avalanche</span> ready · <span class="ok">Turf Network</span> inside
        </pre>
      </section>
      <pre class="art">
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                                                                                                                               
                             ###                                                                                               
                             ***##                                                                                             
                              @%**#                                                                                            
                                @%**                                                                                           
                                  @***                                                                                         
                                   ***                                                                                         
                                   @@**#    #*****       ***                ********#                                          
                                     **# *************** *****            **************                                       
                                     ***************#*** *******        ********************                                   
                                   ********@%*******#*** ********#      #*********************                                
                                   ********%#**************************************************#                               
                                   *****************##*****************************************#                               
                                 *****************@@@********************@***********************#                             
                                 @@***********#%%%@ ****%****************************************#                             
                                   ******#####%    %****@**********####****###*******************#                             
                                   %%%%%%         ****%% %%%%%%%%**#  @%#**** %%%%***********#%%%***                            
                                              #*******@         @****   #******  @***********#  @**#                            
                                            #*******#             @**%    @@****#  @@@@@@****#                                  
                                           ****@                   ****     *******      #******                                 
                                         *#**@                     @***      @****#       @#***##                                
                                         **%@                       ***        @**#         @****#                               
                                      ***@@                         *%@         @@**#        @@@@******                         
                                   ****%%                          **%            *****          @@@@@@                         
                                  @####                            ##@            ####%                                         
      </pre>
    </div>
  </body>
</html>
"#);

    (StatusCode::OK, Html(html))
}

/// `GET /supported`: Lists the x402 payment schemes and networks supported by this facilitator.
///
/// Facilitators may expose this to help clients dynamically configure their payment requests
/// based on available network and scheme support.
#[instrument(skip_all)]
pub async fn get_supported<A>(State(facilitator): State<A>) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.supported().await {
        Ok(supported) => (StatusCode::OK, Json(json!(supported))).into_response(),
        Err(error) => error.into_response(),
    }
}

#[instrument(skip_all)]
pub async fn get_health<A>(State(facilitator): State<A>) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    get_supported(State(facilitator)).await
}

/// `POST /verify`: Facilitator-side verification of a proposed x402 payment.
///
/// This endpoint checks whether a given payment payload satisfies the declared
/// [`PaymentRequirements`], including signature validity, scheme match, and fund sufficiency.
///
/// Responds with a [`VerifyResponse`] indicating whether the payment can be accepted.
#[instrument(skip_all)]
pub async fn post_verify<A>(
    State(facilitator): State<A>,
    Json(body): Json<VerifyRequest>,
) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.verify(&body).await {
        Ok(valid_response) => (StatusCode::OK, Json(valid_response)).into_response(),
        Err(error) => {
            tracing::warn!(
                error = ?error,
                body = %serde_json::to_string(&body).unwrap_or_else(|_| "<can-not-serialize>".to_string()),
                "Verification failed"
            );
            error.into_response()
        }
    }
}

/// `POST /settle`: Facilitator-side execution of a valid x402 payment on-chain.
///
/// Given a valid [`SettleRequest`], this endpoint attempts to execute the payment
/// via ERC-3009 `transferWithAuthorization`, and returns a [`SettleResponse`] with transaction details.
///
/// This endpoint is typically called after a successful `/verify` step.
#[instrument(skip_all)]
pub async fn post_settle<A>(
    State(facilitator): State<A>,
    Json(body): Json<SettleRequest>,
) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.settle(&body).await {
        Ok(valid_response) => (StatusCode::OK, Json(valid_response)).into_response(),
        Err(error) => {
            tracing::warn!(
                error = ?error,
                body = %serde_json::to_string(&body).unwrap_or_else(|_| "<can-not-serialize>".to_string()),
                "Settlement failed"
            );
            error.into_response()
        }
    }
}

fn invalid_schema(payer: Option<MixedAddress>) -> VerifyResponse {
    VerifyResponse::invalid(payer, FacilitatorErrorReason::InvalidScheme)
}

impl IntoResponse for FacilitatorLocalError {
    fn into_response(self) -> Response {
        let error = self;

        let bad_request = (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
            }),
        )
            .into_response();

        match error {
            FacilitatorLocalError::SchemeMismatch(payer, ..) => {
                (StatusCode::OK, Json(invalid_schema(payer))).into_response()
            }
            FacilitatorLocalError::ReceiverMismatch(payer, ..)
            | FacilitatorLocalError::InvalidSignature(payer, ..)
            | FacilitatorLocalError::InvalidTiming(payer, ..)
            | FacilitatorLocalError::InsufficientValue(payer) => {
                (StatusCode::OK, Json(invalid_schema(Some(payer)))).into_response()
            }
            FacilitatorLocalError::NetworkMismatch(payer, ..)
            | FacilitatorLocalError::UnsupportedNetwork(payer) => (
                StatusCode::OK,
                Json(VerifyResponse::invalid(
                    payer,
                    FacilitatorErrorReason::InvalidNetwork,
                )),
            )
                .into_response(),
            FacilitatorLocalError::ContractCall(..)
            | FacilitatorLocalError::InvalidAddress(..)
            | FacilitatorLocalError::ClockError(_) => bad_request,
            FacilitatorLocalError::DecodingError(reason) => (
                StatusCode::OK,
                Json(VerifyResponse::invalid(
                    None,
                    FacilitatorErrorReason::FreeForm(reason),
                )),
            )
                .into_response(),
            FacilitatorLocalError::InsufficientFunds(payer) => (
                StatusCode::OK,
                Json(VerifyResponse::invalid(
                    Some(payer),
                    FacilitatorErrorReason::InsufficientFunds,
                )),
            )
                .into_response(),
        }
    }
}
