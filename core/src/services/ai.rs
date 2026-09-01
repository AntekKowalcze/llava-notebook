
use std::println;

use serde::{Deserialize, Serialize};

use crate::online_auth::AccessToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPromptContext {
    pub document: String,
    pub selection: String,
    pub instruction: String,
}



pub async fn send_ai_request(client: reqwest::Client, ctx: AiPromptContext, access_token: AccessToken) -> Result<reqwest::Response, crate::errors::Error>{
   let res = client.post(format!("{}ai/", crate::constants::SERVER_ADDRESS)).bearer_auth(access_token.0).json(&ctx).send().await.map_err(|e| {
    tracing::error!(
         task = "ai request",
                status = "error",
                error = ?e,
                "failed to send ai request"
    );
      crate::errors::Error::AiError
   })?;
  
   println!("{:?}", res);
   if !res.status().is_success(){
    let code = res.status().as_u16();
    if code == 401 {
        return Err(crate::errors::Error::OnlineSessionExpired);
    }
     return Err(crate::errors::Error::AiError)
   }

    Ok(res)
}