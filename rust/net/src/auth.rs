//
// Copyright 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//
use std::time::SystemTime;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::infra::HttpRequestDecorator;
use crate::utils::basic_authorization;

pub trait HttpBasicAuth {
    fn username(&self) -> &str;
    fn password(&self) -> &str;
}

impl<T: HttpBasicAuth> From<T> for HttpRequestDecorator {
    fn from(value: T) -> Self {
        HttpRequestDecorator::HeaderAuth(basic_authorization(value.username(), value.password()))
    }
}

/// username and password as returned by the chat server's /auth endpoints.
/// - username is a "hex(uid)"
/// - password is a "timestamp:hex(otp(uid, timestamp, secret))"
#[derive(Clone)]
pub struct Auth {
    pub username: String,
    pub password: String,
}

impl Auth {
    pub fn from_uid_and_secret(uid: [u8; 16], secret: [u8; 32]) -> Self {
        let username = hex::encode(uid);
        let password = Self::otp(&username, &secret, SystemTime::now());
        Self { username, password }
    }

    const OTP_LEN: usize = 20;
    fn otp(username: &str, secret: &[u8], now: SystemTime) -> String {
        let ts = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mac_input = format!("{}:{}", &username, ts);
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(mac_input.as_bytes());

        let digest = mac.finalize().into_bytes();
        let mut khex = hex::encode(digest);
        khex.truncate(Self::OTP_LEN);
        format!("{}:{}", ts, khex)
    }
}

impl HttpBasicAuth for Auth {
    fn username(&self) -> &str {
        &self.username
    }

    fn password(&self) -> &str {
        &self.password
    }
}
